mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, PASSWORD};
use serde_json::json;

#[tokio::test]
async fn signup_rejects_personal_email() {
    let app = TestApp::new().await;
    let (_, res) = app.signup("someone@gmail.com").await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.code(), "not_school_email");
    assert!(res.json["error"].get("fields").is_none());
    let (_, res) = app.signup("someone@yahoo.co.uk").await;
    assert_eq!(res.code(), "not_school_email");
}

#[tokio::test]
async fn signup_on_approved_domain_links_school_and_sends_code() {
    let app = TestApp::new().await;
    let (c, res) = app.signup("Maya.R@G.UCLA.edu").await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
    let user = &res.json["user"];
    assert_eq!(user["email"], "maya.r@g.ucla.edu");
    assert_eq!(user["status"], "pending_email");
    assert_eq!(user["needs_school"], false);
    assert_eq!(user["school"]["name"], "UCLA");
    assert_eq!(user["display_name"], "Test S.");
    let cookie = res.headers.get("set-cookie").unwrap().to_str().unwrap();
    assert!(cookie.starts_with("corner_session="));
    for part in ["HttpOnly", "SameSite=Lax", "Path=/", "Max-Age=2592000"] {
        assert!(cookie.contains(part), "{cookie}");
    }
    assert!(!cookie.contains("Secure"));
    assert_eq!(app.code_for("maya.r@g.ucla.edu").len(), 6);
    // The session works right away.
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "pending_email");
}

#[tokio::test]
async fn signup_validation_too_young_and_duplicate() {
    let app = TestApp::new().await;
    let mut c = app.client();
    let res = c
        .post("/api/auth/signup", json!({ "email": "nope", "password": "short", "first_name": "", "last_name": "X" }))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.code(), "validation");
    let fields = &res.json["error"]["fields"];
    for f in ["email", "password", "first_name", "birth_year"] {
        assert!(fields.get(f).is_some(), "missing field error for {f}: {fields}");
    }
    assert!(fields.get("last_name").is_none());

    let this_year = chrono::Datelike::year(&chrono::Utc::now());
    let res = c
        .post(
            "/api/auth/signup",
            json!({ "email": "kid@ucla.edu", "password": PASSWORD, "first_name": "K", "last_name": "D", "birth_year": this_year - 12, "enrolled_on": "2024-09" }),
        )
        .await;
    assert_eq!(res.code(), "too_young");
    let res = c
        .post(
            "/api/auth/signup",
            json!({ "email": "teen@ucla.edu", "password": PASSWORD, "first_name": "T", "last_name": "N", "birth_year": this_year - 13, "enrolled_on": "2024-09" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);

    let (_, res) = app.signup("dup@ucla.edu").await;
    assert_eq!(res.status, StatusCode::CREATED);
    let (_, res) = app.signup("DUP@ucla.edu").await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.code(), "email_taken");
}

#[tokio::test]
async fn unknown_edu_picks_school_and_auto_approves() {
    let app = TestApp::new().await;
    let (mut c, res) = app.signup("li@cs.stateu.edu").await;
    assert_eq!(res.json["user"]["needs_school"], true);
    assert!(res.json["user"]["school"].is_null());

    // Verifying before picking a school leaves the user waiting on a school.
    let code = app.code_for("li@cs.stateu.edu");
    let res = c.post("/api/auth/verify", json!({ "code": code })).await;
    assert_eq!(res.json["user"]["status"], "pending_school");

    let res = c
        .post(
            "/api/auth/school",
            json!({ "new_school": { "name": "State University", "kind": "university", "city": "Springfield", "state": "il" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["user"]["status"], "verified");
    assert_eq!(res.json["user"]["needs_school"], false);
    assert_eq!(res.json["user"]["school"]["state"], "IL");

    let status: String = sqlx::query_scalar("SELECT status FROM school_domains WHERE domain = 'stateu.edu'")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "approved");

    // Picking again is not allowed.
    let res = c.post("/api/auth/school", json!({ "school_id": app.ids.ucla })).await;
    assert_eq!(res.code(), "forbidden");

    // A classmate on another subdomain is linked automatically.
    let (_, res) = app.signup("pat@stateu.edu").await;
    assert_eq!(res.json["user"]["needs_school"], false);
    assert_eq!(res.json["user"]["school"]["name"], "State University");
}

#[tokio::test]
async fn unknown_domain_goes_through_admin_queue() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;

    let (mut c, res) = app.signup("ava@stu.bhusd.org").await;
    assert_eq!(res.json["user"]["needs_school"], true);
    let res = c
        .post(
            "/api/auth/school",
            json!({ "new_school": { "name": "Beverly Hills High School", "kind": "high_school", "city": "Beverly Hills", "state": "CA" } }),
        )
        .await;
    assert_eq!(res.json["user"]["status"], "pending_email");
    let code = app.code_for("ava@stu.bhusd.org");
    let res = c.post("/api/auth/verify", json!({ "code": code })).await;
    assert_eq!(res.json["user"]["status"], "pending_school");

    // A second student names a different school; the existing request wins.
    let (mut c2, _) = app.signup("max@stu.bhusd.org").await;
    let res = c2.post("/api/auth/school", json!({ "school_id": app.ids.uhs })).await;
    assert_eq!(res.json["user"]["school"]["name"], "Beverly Hills High School");
    let code = app.code_for("max@stu.bhusd.org");
    c2.post("/api/auth/verify", json!({ "code": code })).await;

    let list = admin.get("/api/admin/domains?status=pending").await;
    assert_eq!(list.status, StatusCode::OK);
    let item = &list.json["items"][0];
    assert_eq!(item["domain"], "stu.bhusd.org");
    assert_eq!(item["waiting_count"], 2);
    assert_eq!(item["school"]["name"], "Beverly Hills High School");
    assert_eq!(item["flags"], json!([]));

    let res = admin.post("/api/admin/domains/stu.bhusd.org/approve", json!({})).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["status"], "approved");
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "verified");
    assert_eq!(app.mailer.last_to("ava@stu.bhusd.org").unwrap().subject, "You're in");

    // Undo puts both the domain and the users back.
    let res = admin.post("/api/admin/domains/stu.bhusd.org/reset", json!({})).await;
    assert_eq!(res.json["status"], "pending");
    assert_eq!(res.json["waiting_count"], 2);
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "pending_school");

    let res = admin
        .post("/api/admin/domains/stu.bhusd.org/reject", json!({ "reason": "Not a student domain" }))
        .await;
    assert_eq!(res.json["status"], "rejected");
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "rejected");
    assert!(app.mailer.last_to("ava@stu.bhusd.org").unwrap().text.contains("Not a student domain"));

    // A user who verifies after the rejection lands in rejected too.
    let (mut c3, _) = app.signup("zed@stu.bhusd.org").await;
    c3.post("/api/auth/school", json!({ "school_id": app.ids.ucla })).await;
    let code = app.code_for("zed@stu.bhusd.org");
    let res = c3.post("/api/auth/verify", json!({ "code": code })).await;
    assert_eq!(res.json["user"]["status"], "rejected");

    let res = admin.post("/api/admin/domains/nope.example/approve", json!({})).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn verify_wrong_code_expiry_and_lockout() {
    let app = TestApp::new().await;
    let (mut c, _) = app.signup("v@ucla.edu").await;
    let code = app.code_for("v@ucla.edu");
    let wrong = if code == "000000" { "111111" } else { "000000" };

    let res = c.post("/api/auth/verify", json!({ "code": "12ab" })).await;
    assert_eq!(res.code(), "validation");

    let res = c.post("/api/auth/verify", json!({ "code": wrong })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.code(), "invalid_code");
    assert_eq!(res.json["error"]["attempts_left"], 4);

    // Expiry
    sqlx::query("UPDATE verification_codes SET expires_at = now() - interval '1 minute'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post("/api/auth/verify", json!({ "code": code })).await;
    assert_eq!(res.code(), "code_expired");
    sqlx::query("UPDATE verification_codes SET expires_at = now() + interval '10 minutes'")
        .execute(&app.pool)
        .await
        .unwrap();

    for left in (0..4).rev() {
        let res = c.post("/api/auth/verify", json!({ "code": wrong })).await;
        assert_eq!(res.json["error"]["attempts_left"], left);
    }
    // After five wrong tries even the right code is refused.
    let res = c.post("/api/auth/verify", json!({ "code": code })).await;
    assert_eq!(res.code(), "too_many_attempts");

    // A new code resets the attempts.
    sqlx::query("UPDATE verification_codes SET created_at = now() - interval '2 minutes'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post("/api/auth/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let fresh = app.code_for("v@ucla.edu");
    let res = c.post("/api/auth/verify", json!({ "code": fresh })).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["user"]["status"], "verified");

    let res = c.post("/api/auth/verify", json!({ "code": fresh })).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.code(), "already_verified");
}

#[tokio::test]
async fn resend_has_cooldown_and_invalidates_old_code() {
    let app = TestApp::new().await;
    let (mut c, _) = app.signup("r@ucla.edu").await;
    let first = app.code_for("r@ucla.edu");

    let res = c.post("/api/auth/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(res.code(), "rate_limited");
    let retry = res.json["error"]["retry_after"].as_u64().unwrap();
    assert!((1..=60).contains(&retry));
    assert_eq!(res.headers.get("retry-after").unwrap().to_str().unwrap(), retry.to_string());

    sqlx::query("UPDATE verification_codes SET created_at = now() - interval '61 seconds'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post("/api/auth/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let second = app.code_for("r@ucla.edu");
    if first != second {
        let res = c.post("/api/auth/verify", json!({ "code": first })).await;
        assert_eq!(res.code(), "invalid_code");
    }
    let res = c.post("/api/auth/verify", json!({ "code": second })).await;
    assert_eq!(res.json["user"]["status"], "verified");
}

#[tokio::test]
async fn login_and_logout() {
    let app = TestApp::new().await;
    app.verified("l@ucla.edu").await;

    let mut c = app.client();
    let res = c.post("/api/auth/login", json!({ "email": "l@ucla.edu", "password": "wrong-password" })).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.code(), "invalid_credentials");
    let wrong_pw_msg = res.json["error"]["message"].clone();
    let res = c.post("/api/auth/login", json!({ "email": "ghost@ucla.edu", "password": PASSWORD })).await;
    assert_eq!(res.code(), "invalid_credentials");
    assert_eq!(res.json["error"]["message"], wrong_pw_msg);

    assert_eq!(c.get("/api/me").await.status, StatusCode::UNAUTHORIZED);
    let res = c.post("/api/auth/login", json!({ "email": " L@UCLA.edu ", "password": PASSWORD })).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["user"]["status"], "verified");
    let old_cookie = c.cookie.clone();
    assert_eq!(c.get("/api/me").await.status, StatusCode::OK);

    let res = c.patch_me().await;
    assert_eq!(res.json["user"]["display_name"], "Lee P.");

    let res = c.post("/api/auth/logout", json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.headers.get("set-cookie").unwrap().to_str().unwrap().contains("Max-Age=0"));
    c.cookie = old_cookie;
    assert_eq!(c.get("/api/me").await.code(), "unauthenticated");

    sqlx::query("UPDATE users SET status = 'suspended' WHERE email = 'l@ucla.edu'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post("/api/auth/login", json!({ "email": "l@ucla.edu", "password": PASSWORD })).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.code(), "suspended");
}

trait PatchMe {
    async fn patch_me(&mut self) -> common::Resp;
}

impl PatchMe for common::Client<'_> {
    async fn patch_me(&mut self) -> common::Resp {
        self.json(Method::PATCH, "/api/me", json!({ "first_name": "Lee", "last_name": "Park" })).await
    }
}

#[tokio::test]
async fn csrf_requires_json_and_allowed_origin() {
    let app = TestApp::new().await;
    let mut c = app.client();
    let body = json!({ "email": "x@ucla.edu", "password": PASSWORD });

    let res = c.send(Method::POST, "/api/auth/login", Some(body.clone()), &[]).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.code(), "forbidden_origin");

    let res = c
        .send(Method::POST, "/api/auth/login", Some(body.clone()), &[("content-type", "text/plain")])
        .await;
    assert_eq!(res.code(), "forbidden_origin");

    let res = c
        .send(
            Method::POST,
            "/api/auth/login",
            Some(body.clone()),
            &[("content-type", "application/json"), ("origin", "https://evil.example")],
        )
        .await;
    assert_eq!(res.code(), "forbidden_origin");

    let res = c
        .send(
            Method::POST,
            "/api/auth/login",
            Some(body),
            &[("content-type", "application/json; charset=utf-8"), ("origin", "http://localhost:3000")],
        )
        .await;
    assert_eq!(res.code(), "invalid_credentials");

    // GETs are never blocked.
    assert_eq!(c.get("/api/schools").await.status, StatusCode::OK);
}

#[tokio::test]
async fn schools_search() {
    let app = TestApp::new().await;
    let c = app.client();
    let res = c.get("/api/schools?q=univ").await;
    let names: Vec<&str> = res.json["items"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["University High School"]);
    let res = c.get("/api/schools?q=monica").await;
    assert_eq!(res.json["items"][0]["name"], "Santa Monica College");
    assert!(res.json["items"][0]["lat"].is_number());
    let res = c.get("/api/schools?kind=high_school").await;
    assert_eq!(res.json["items"].as_array().unwrap().len(), 1);
}
