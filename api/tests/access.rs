//! Enrollment-based access, saved places, review edits and mail failures.

mod common;

use axum::http::{Method, StatusCode};
use chrono::{Datelike, Months, Utc};
use common::{TestApp, PASSWORD};
use serde_json::json;

const BODY: &str = "The spicy miso is worth the line. Go before six or you will wait half an hour.";

fn month_offset(months: i32) -> String {
    let first = Utc::now().date_naive().with_day(1).unwrap();
    let d = if months >= 0 {
        first.checked_add_months(Months::new(months as u32)).unwrap()
    } else {
        first.checked_sub_months(Months::new((-months) as u32)).unwrap()
    };
    d.format("%Y-%m").to_string()
}

#[tokio::test]
async fn signup_enrollment_validation() {
    let app = TestApp::new().await;
    let mut c = app.client();
    let res = c
        .post(
            "/api/auth/signup",
            json!({ "email": "a@ucla.edu", "password": PASSWORD, "first_name": "A", "last_name": "B", "birth_year": 2005 }),
        )
        .await;
    assert_eq!(res.code(), "validation");
    assert!(res.json["error"]["fields"]["enrolled_on"].is_string());

    for bad in ["2024-9", "2024-13", "September", &month_offset(13)] {
        let (_, res) = app.signup_enrolled("b@ucla.edu", bad).await;
        assert_eq!(res.code(), "validation", "{bad}");
        assert!(res.json["error"]["fields"]["enrolled_on"].is_string(), "{bad}");
    }

    // access_until = the 1st of this month, which is <= today.
    let (_, res) = app.signup_enrolled("old@ucla.edu", &month_offset(-60)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.code(), "enrollment_too_old");
    assert_eq!(
        res.json["error"]["message"],
        "Corner is for current students. Access lasts 5 years from when you started."
    );

    let (_, res) = app.signup_enrolled("fine@ucla.edu", &month_offset(-59)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
    let (_, res) = app.signup_enrolled("future@ucla.edu", &month_offset(12)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);

    let (_, res) = app.signup_enrolled("maya@ucla.edu", "2024-09").await;
    assert_eq!(res.json["user"]["enrolled_on"], "2024-09");
    assert_eq!(res.json["user"]["access_until"], "2029-09-01");
    assert!(res.json["user"]["pending_school_change"].is_null());
}

#[tokio::test]
async fn expired_users_browse_and_save_but_cannot_post() {
    let app = TestApp::new().await;
    let place = app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, None).await;
    let mut author = app.verified("author@ucla.edu").await;
    let res = author.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 5, "body": BODY })).await;
    let review_id = res.json["review"]["id"].as_str().unwrap().to_string();

    let mut c = app.verified("grad@ucla.edu").await;
    // One month left: still verified.
    app.set_access_until("grad@ucla.edu", 1).await;
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "verified");

    // access_until is the 1st of this month: expired.
    app.set_access_until("grad@ucla.edu", 0).await;
    let me = c.get("/api/me").await;
    assert_eq!(me.json["user"]["status"], "expired");
    let res = c.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 4, "body": BODY })).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.code(), "access_expired");
    let res = c.json(Method::PUT, &format!("/api/reviews/{review_id}/vote"), json!({})).await;
    assert_eq!(res.code(), "access_expired");
    let res = c
        .post("/api/reports", json!({ "target_type": "place", "target_id": place, "reason": "wrong_info" }))
        .await;
    assert_eq!(res.code(), "access_expired");

    // Browsing, saving and signing in still work.
    assert_eq!(c.get("/api/feed").await.status, StatusCode::OK);
    assert_eq!(c.get(&format!("/api/places/{place}")).await.status, StatusCode::OK);
    let res = c.json(Method::PUT, &format!("/api/places/{place}/save"), json!({})).await;
    assert_eq!(res.json, json!({ "saved": true }));
    let mut fresh = app.client();
    let res = fresh.post("/api/auth/login", json!({ "email": "grad@ucla.edu", "password": PASSWORD })).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["user"]["status"], "expired");

    // Admins never expire.
    let admin = app.admin().await;
    app.set_access_until("admin@ucla.edu", -12).await;
    assert_eq!(admin.get("/api/me").await.json["user"]["status"], "verified");
    assert_eq!(admin.get("/api/admin/domains").await.status, StatusCode::OK);
}

#[tokio::test]
async fn saved_places_are_idempotent_and_listed() {
    let app = TestApp::new().await;
    let a = app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, None).await;
    let b = app.insert_place("Lindbrook Coffee", "coffee", 34.0598, -118.4440, None).await;
    // Saving doesn't need a verified status.
    let (mut c, _) = app.signup("saver@ucla.edu").await;

    for _ in 0..2 {
        let res = c.json(Method::PUT, &format!("/api/places/{a}/save"), json!({})).await;
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.json, json!({ "saved": true }));
    }
    c.json(Method::PUT, &format!("/api/places/{b}/save"), json!({})).await;

    let list = c.get("/api/me/saved").await;
    assert_eq!(list.status, StatusCode::OK);
    let names: Vec<&str> = list.json["items"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Lindbrook Coffee", "Gayley Noodle Bar"]);
    assert!(list.json["next_cursor"].is_null());
    let page = c.get("/api/me/saved?limit=1").await;
    assert!(page.json["next_cursor"].is_string());

    assert_eq!(c.get(&format!("/api/places/{a}")).await.json["place"]["viewer_saved"], true);
    assert_eq!(app.client().get(&format!("/api/places/{a}")).await.json["place"]["viewer_saved"], false);

    for _ in 0..2 {
        let res = c.json(Method::DELETE, &format!("/api/places/{a}/save"), json!({})).await;
        assert_eq!(res.json, json!({ "saved": false }));
    }
    assert_eq!(c.get(&format!("/api/places/{a}")).await.json["place"]["viewer_saved"], false);

    // Hidden places drop out of the list.
    sqlx::query("UPDATE places SET status = 'hidden' WHERE id = $1").bind(b).execute(&app.pool).await.unwrap();
    assert_eq!(c.get("/api/me/saved").await.json["items"].as_array().unwrap().len(), 0);

    let mut anon = app.client();
    assert_eq!(anon.json(Method::PUT, &format!("/api/places/{a}/save"), json!({})).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anon.get("/api/me/saved").await.status, StatusCode::UNAUTHORIZED);
    let missing = uuid::Uuid::new_v4();
    assert_eq!(c.json(Method::PUT, &format!("/api/places/{missing}/save"), json!({})).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn review_edit_window_and_edited_at() {
    let app = TestApp::new().await;
    let place = app.insert_place("Strathmore Tacos", "food", 34.0693, -118.4513, None).await;
    let mut c = app.verified("editor@ucla.edu").await;
    let res = c
        .post(&format!("/api/places/{place}/reviews"), json!({ "rating": 4, "body": BODY, "used_deal": false }))
        .await;
    let review = &res.json["review"];
    let id = review["id"].as_str().unwrap().to_string();
    assert!(review["edited_at"].is_null());
    let created = chrono::DateTime::parse_from_rfc3339(review["created_at"].as_str().unwrap()).unwrap();
    let until = chrono::DateTime::parse_from_rfc3339(review["editable_until"].as_str().unwrap()).unwrap();
    assert_eq!(until - created, chrono::Duration::minutes(15));

    // Others see the review without editable_until.
    let other = app.client().get(&format!("/api/reviews/{id}")).await;
    assert_eq!(other.status, StatusCode::OK);
    assert!(other.json["review"]["editable_until"].is_null());
    assert_eq!(other.json["review"]["is_mine"], false);

    let path = format!("/api/reviews/{id}");
    let res = c.json(Method::PATCH, &path, json!({ "used_deal": true, "visited_month": "2026-01" })).await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(res.json["review"]["edited_at"].is_null(), "meta-only edits don't count");
    let res = c.json(Method::PATCH, &path, json!({ "rating": 4, "body": format!("  {BODY}  ") })).await;
    assert!(res.json["review"]["edited_at"].is_null(), "unchanged rating and body don't count");
    let res = c.json(Method::PATCH, &path, json!({ "rating": 3 })).await;
    assert!(res.json["review"]["edited_at"].is_string());

    let res = c.json(Method::PATCH, &path, json!({ "body": "short" })).await;
    assert_eq!(res.code(), "validation");

    sqlx::query("UPDATE reviews SET created_at = now() - interval '16 minutes' WHERE id = $1::uuid")
        .bind(&id)
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.json(Method::PATCH, &path, json!({ "rating": 5 })).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.code(), "edit_window_closed");
    assert_eq!(res.json["error"]["message"], "Reviews can be edited for 15 minutes after posting.");

    // GET of a deleted review: the author can still see it during the restore window.
    c.json(Method::DELETE, &path, json!({})).await;
    assert_eq!(c.get(&path).await.status, StatusCode::OK);
    assert_eq!(app.client().get(&path).await.status, StatusCode::NOT_FOUND);
    sqlx::query("UPDATE reviews SET deleted_at = now() - interval '6 minutes'").execute(&app.pool).await.unwrap();
    assert_eq!(c.get(&path).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn mail_failures() {
    let app = TestApp::new().await;
    app.mailer.set_failing(true);
    // Signup still succeeds.
    let (mut c, res) = app.signup("unlucky@ucla.edu").await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert!(app.mailer.last_to("unlucky@ucla.edu").is_none());

    sqlx::query("UPDATE verification_codes SET created_at = now() - interval '2 minutes'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post("/api/auth/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::BAD_GATEWAY);
    assert_eq!(res.code(), "mail_failed");
    assert_eq!(res.json["error"]["message"], "We couldn't send the email. Try again in a minute.");

    // The failed resend doesn't start a new cooldown.
    app.mailer.set_failing(false);
    let res = c.post("/api/auth/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let code = app.code_for("unlucky@ucla.edu");
    let res = c.post("/api/auth/verify", json!({ "code": code })).await;
    assert_eq!(res.json["user"]["status"], "verified");

    app.mailer.set_failing(true);
    let res = c.post("/api/me/school-change", json!({ "email": "unlucky@student.smc.edu", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.status, StatusCode::BAD_GATEWAY);
    assert_eq!(res.code(), "mail_failed");
    assert!(c.get("/api/me").await.json["user"]["pending_school_change"].is_null());
}
