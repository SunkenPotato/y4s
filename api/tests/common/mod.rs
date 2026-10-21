//! Test harness: a fresh Postgres database per test (on the dev server, port 5433), the router
//! in-process, an in-memory mailer and a stub geocoder.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::Router;
use corner_api::config::Config;
use corner_api::geocode::StubGeocoder;
use corner_api::mail::MemoryMailer;
use corner_api::{app, db, AppState};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

pub const PASSWORD: &str = "correct-horse-battery";
pub const KNOWN_ADDRESS: &str = "1 test plaza, los angeles, ca 90024";

fn server_url() -> String {
    std::env::var("TEST_DATABASE_SERVER").unwrap_or_else(|_| "postgres://corner:corner@localhost:5433".into())
}

pub struct Ids {
    pub ucla: Uuid,
    pub smc: Uuid,
    pub uhs: Uuid,
}

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    pub mailer: MemoryMailer,
    pub ids: Ids,
    db_name: String,
}

impl TestApp {
    pub async fn new() -> Self {
        let db_name = format!("corner_test_{}", Uuid::new_v4().simple());
        let mut admin = PgConnection::connect(&format!("{}/postgres", server_url()))
            .await
            .expect("connect to test postgres on :5433");
        admin
            .execute(format!("CREATE DATABASE {db_name}").as_str())
            .await
            .expect("create test db");
        admin.close().await.ok();

        let url = format!("{}/{db_name}", server_url());
        let pool = db::connect(&url).await.expect("connect test db");
        db::migrate(&pool).await.expect("migrate");

        let mailer = MemoryMailer::default();
        let mut known = HashMap::new();
        known.insert(KNOWN_ADDRESS.to_string(), (34.0640, -118.4450));
        let geocoder = StubGeocoder { known, fallback: None };
        let state = AppState::new(pool.clone(), Config::for_tests(&url), Arc::new(mailer.clone()), Arc::new(geocoder));
        let router = app(state);

        let ids = Ids {
            ucla: Uuid::new_v4(),
            smc: Uuid::new_v4(),
            uhs: Uuid::new_v4(),
        };
        for (id, name, kind, city, lat, lng, domain) in [
            (ids.ucla, "UCLA", "university", "Los Angeles", 34.0689, -118.4452, "ucla.edu"),
            (ids.smc, "Santa Monica College", "university", "Santa Monica", 34.0168, -118.4707, "smc.edu"),
            (ids.uhs, "University High School", "high_school", "Los Angeles", 34.0436, -118.4560, "students.lausd.net"),
        ] {
            sqlx::query("INSERT INTO schools (id, name, kind, city, state, lat, lng) VALUES ($1, $2, $3, $4, 'CA', $5, $6)")
                .bind(id)
                .bind(name)
                .bind(kind)
                .bind(city)
                .bind(lat)
                .bind(lng)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO school_domains (domain, school_id, status, decided_at) VALUES ($1, $2, 'approved', now())")
                .bind(domain)
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }

        Self {
            router,
            pool,
            mailer,
            ids,
            db_name,
        }
    }

    pub fn client(&self) -> Client<'_> {
        Client { app: self, cookie: None }
    }

    /// The most recent 6-digit code emailed to `email`.
    pub fn code_for(&self, email: &str) -> String {
        let mail = self.mailer.last_to(email).expect("an email was sent");
        mail.subject
            .rsplit(' ')
            .next()
            .filter(|c| c.len() == 6)
            .expect("code in subject")
            .to_string()
    }

    pub async fn signup(&self, email: &str) -> (Client<'_>, Resp) {
        self.signup_enrolled(email, "2024-09").await
    }

    pub async fn signup_enrolled(&self, email: &str, enrolled_on: &str) -> (Client<'_>, Resp) {
        let mut c = self.client();
        let res = c
            .post(
                "/api/auth/signup",
                json!({ "email": email, "password": PASSWORD, "first_name": "Test", "last_name": "Student",
                        "birth_year": 2005, "enrolled_on": enrolled_on }),
            )
            .await;
        (c, res)
    }

    /// Moves a user's enrollment so `access_until` lands `months_from_now` months from the 1st of this month.
    pub async fn set_access_until(&self, email: &str, months_from_now: i32) {
        sqlx::query(
            "UPDATE users SET enrolled_on = (date_trunc('month', now())::date
                 + make_interval(months => $2 - 60))::date WHERE email = $1",
        )
        .bind(email)
        .bind(months_from_now)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    /// Signs up on an approved domain and enters the code.
    pub async fn verified(&self, email: &str) -> Client<'_> {
        let (mut c, res) = self.signup(email).await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
        let code = self.code_for(email);
        let res = c.post("/api/auth/verify", json!({ "code": code })).await;
        assert_eq!(res.json["user"]["status"], "verified", "{}", res.json);
        c
    }

    /// `admin@ucla.edu` is in the test config's ADMIN_EMAILS.
    pub async fn admin(&self) -> Client<'_> {
        let c = self.verified("admin@ucla.edu").await;
        let me = c.get("/api/me").await;
        assert_eq!(me.json["user"]["role"], "admin");
        c
    }

    pub async fn insert_place(&self, name: &str, category: &str, lat: f64, lng: f64, deal: Option<&str>) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO places (id, kind, name, category, address, lat, lng, student_discount, status)
             VALUES ($1, 'business', $2, $3, '100 Test St, Los Angeles, CA', $4, $5, $6, 'active')",
        )
        .bind(id)
        .bind(name)
        .bind(category)
        .bind(lat)
        .bind(lng)
        .bind(deal)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Async drop isn't available, so clean up on a separate thread with its own runtime.
        let name = self.db_name.clone();
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(async move {
                if let Ok(mut conn) = PgConnection::connect(&format!("{}/postgres", server_url())).await {
                    let _ = conn
                        .execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
                        .await;
                }
            });
        })
        .join();
    }
}

#[derive(Debug)]
pub struct Resp {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub json: Value,
}

impl Resp {
    pub fn code(&self) -> &str {
        self.json["error"]["code"].as_str().unwrap_or("")
    }
}

pub struct Client<'a> {
    app: &'a TestApp,
    pub cookie: Option<String>,
}

impl Client<'_> {
    pub async fn send(&mut self, method: Method, path: &str, body: Option<Value>, headers: &[(&str, &str)]) -> Resp {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(cookie) = &self.cookie {
            req = req.header(header::COOKIE, cookie);
        }
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let req = req
            .body(match body {
                Some(b) => Body::from(b.to_string()),
                None => Body::empty(),
            })
            .unwrap();
        let res = self.app.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        if let Some(set) = headers.get(header::SET_COOKIE).and_then(|v| v.to_str().ok()) {
            let pair = set.split(';').next().unwrap_or_default().to_string();
            self.cookie = if pair.ends_with('=') { None } else { Some(pair) };
        }
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
        };
        Resp { status, headers, json }
    }

    pub async fn get(&self, path: &str) -> Resp {
        // GETs don't change the cookie; use a throwaway clone so callers can keep `&self`.
        let mut c = Client { app: self.app, cookie: self.cookie.clone() };
        c.send(Method::GET, path, None, &[]).await
    }

    pub async fn json(&mut self, method: Method, path: &str, body: Value) -> Resp {
        self.send(method, path, Some(body), &[("content-type", "application/json")]).await
    }

    pub async fn post(&mut self, path: &str, body: Value) -> Resp {
        self.json(Method::POST, path, body).await
    }
}
