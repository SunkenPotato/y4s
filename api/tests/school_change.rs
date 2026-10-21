//! Changing school, and re-picking a school during signup.

mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, PASSWORD};
use serde_json::json;

const BODY: &str = "The spicy miso is worth the line. Go before six or you will wait half an hour.";

#[tokio::test]
async fn change_to_known_domain_keeps_review_school() {
    let app = TestApp::new().await;
    let a = app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, None).await;
    let b = app.insert_place("Pico Taco Stand", "food", 34.0175, -118.4725, None).await;
    let mut c = app.verified("maya@ucla.edu").await;
    let res = c.post(&format!("/api/places/{a}/reviews"), json!({ "rating": 5, "body": BODY })).await;
    let old_review = res.json["review"]["id"].as_str().unwrap().to_string();
    c.json(Method::PUT, &format!("/api/places/{a}/save"), json!({})).await;

    let res = c
        .post("/api/me/school-change", json!({ "email": "Maya@Student.SMC.edu", "enrolled_on": "2025-09" }))
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let pending = &res.json["user"]["pending_school_change"];
    assert_eq!(pending["email"], "maya@student.smc.edu");
    assert_eq!(pending["needs_school"], false);
    assert_eq!(pending["school"]["name"], "Santa Monica College");
    assert_eq!(pending["enrolled_on"], "2025-09");
    // Nothing changes until the code is entered.
    assert_eq!(res.json["user"]["email"], "maya@ucla.edu");
    assert_eq!(res.json["user"]["status"], "verified");

    let code = app.code_for("maya@student.smc.edu");
    let wrong = if code == "000000" { "111111" } else { "000000" };
    let res = c.post("/api/me/school-change/verify", json!({ "code": wrong })).await;
    assert_eq!(res.code(), "invalid_code");
    assert_eq!(res.json["error"]["attempts_left"], 4);

    let res = c.post("/api/me/school-change/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
    sqlx::query("UPDATE school_changes SET code_sent_at = now() - interval '61 seconds'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post("/api/me/school-change/resend", json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let code = app.code_for("maya@student.smc.edu");

    // A school is known for this domain, so /school isn't allowed.
    let res = c.post("/api/me/school-change/school", json!({ "school_id": app.ids.uhs })).await;
    assert_eq!(res.code(), "forbidden");

    let res = c.post("/api/me/school-change/verify", json!({ "code": code })).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let user = &res.json["user"];
    assert_eq!(user["email"], "maya@student.smc.edu");
    assert_eq!(user["school"]["name"], "Santa Monica College");
    assert_eq!(user["enrolled_on"], "2025-09");
    assert_eq!(user["access_until"], "2030-09-01");
    assert_eq!(user["status"], "verified");
    assert!(user["pending_school_change"].is_null());

    // The old review keeps UCLA; a new one is posted under SMC.
    let old = c.get(&format!("/api/reviews/{old_review}")).await;
    assert_eq!(old.json["review"]["author"]["school_name"], "UCLA");
    let res = c.post(&format!("/api/places/{b}/reviews"), json!({ "rating": 4, "body": BODY })).await;
    assert_eq!(res.json["review"]["author"]["school_name"], "Santa Monica College");
    let quote = app.client().get(&format!("/api/places/{a}")).await;
    assert_eq!(quote.json["place"]["top_quote"]["author"]["school_name"], "UCLA");
    // Saved places carry over.
    assert_eq!(c.get("/api/me/saved").await.json["items"].as_array().unwrap().len(), 1);

    let mut login = app.client();
    let res = login.post("/api/auth/login", json!({ "email": "maya@ucla.edu", "password": PASSWORD })).await;
    assert_eq!(res.code(), "invalid_credentials");
    let res = login.post("/api/auth/login", json!({ "email": "maya@student.smc.edu", "password": PASSWORD })).await;
    assert_eq!(res.status, StatusCode::OK);
}

#[tokio::test]
async fn change_to_unknown_domain_goes_pending() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let mut c = app.verified("hs.grad@students.lausd.net").await;
    let res = c.post("/api/me/school-change", json!({ "email": "kim@newcollege.org", "enrolled_on": "2026-09" })).await;
    assert_eq!(res.json["user"]["pending_school_change"]["needs_school"], true);
    assert!(res.json["user"]["pending_school_change"]["school"].is_null());

    let code = app.code_for("kim@newcollege.org");
    let res = c.post("/api/me/school-change/verify", json!({ "code": code })).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.code(), "school_choice_required");

    let res = c
        .post(
            "/api/me/school-change/school",
            json!({ "new_school": { "name": "New College", "kind": "university", "city": "Oakland", "state": "CA" } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["user"]["pending_school_change"]["needs_school"], false);
    assert_eq!(res.json["user"]["pending_school_change"]["school"]["name"], "New College");
    // Still the old school until verified.
    assert_eq!(res.json["user"]["school"]["name"], "University High School");

    let res = c.post("/api/me/school-change/verify", json!({ "code": code })).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["user"]["status"], "pending_school");
    assert_eq!(res.json["user"]["school"]["name"], "New College");

    let res = admin.post("/api/admin/domains/newcollege.org/approve", json!({})).await;
    assert_eq!(res.json["status"], "approved");
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "verified");
}

#[tokio::test]
async fn change_request_rules_and_cancel() {
    let app = TestApp::new().await;
    app.verified("taken@student.smc.edu").await;
    let mut c = app.verified("mover@ucla.edu").await;

    let res = c.post("/api/me/school-change", json!({ "email": "mover@gmail.com", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.code(), "not_school_email");
    let res = c.post("/api/me/school-change", json!({ "email": "taken@student.smc.edu", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.code(), "email_taken");
    let res = c.post("/api/me/school-change", json!({ "email": "m@student.smc.edu", "enrolled_on": "2015-09" })).await;
    assert_eq!(res.code(), "enrollment_too_old");
    let res = c.post("/api/me/school-change", json!({ "email": "m@student.smc.edu" })).await;
    assert!(res.json["error"]["fields"]["enrolled_on"].is_string());
    let res = c.post("/api/me/school-change", json!({ "email": "mover@ucla.edu", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.code(), "validation");

    let res = c.post("/api/me/school-change", json!({ "email": "m@student.smc.edu", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.status, StatusCode::OK);
    let res = c.json(Method::DELETE, "/api/me/school-change", json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(c.get("/api/me").await.json["user"]["pending_school_change"].is_null());
    let res = c.post("/api/me/school-change/verify", json!({ "code": "123456" })).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    // Only verified (or expired) users can change school.
    let (mut fresh, _) = app.signup("fresh@ucla.edu").await;
    let res = fresh.post("/api/me/school-change", json!({ "email": "f@student.smc.edu", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.code(), "forbidden");

    // An expired user can move to a new school and post again.
    app.set_access_until("mover@ucla.edu", 0).await;
    assert_eq!(c.get("/api/me").await.json["user"]["status"], "expired");
    sqlx::query("UPDATE school_changes SET code_sent_at = now() - interval '2 minutes'").execute(&app.pool).await.unwrap();
    let res = c.post("/api/me/school-change", json!({ "email": "m@student.smc.edu", "enrolled_on": "2025-09" })).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let code = app.code_for("m@student.smc.edu");
    let res = c.post("/api/me/school-change/verify", json!({ "code": code })).await;
    assert_eq!(res.json["user"]["status"], "verified");
}

#[tokio::test]
async fn repick_school_during_signup() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;

    // The only student waiting can move the request.
    let (mut a, _) = app.signup("a@stu.example.org").await;
    a.post("/api/auth/school", json!({ "school_id": app.ids.uhs })).await;
    let res = a.post("/api/auth/school", json!({ "school_id": app.ids.smc })).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["user"]["school"]["name"], "Santa Monica College");
    let domains = admin.get("/api/admin/domains?status=pending").await;
    assert_eq!(domains.json["items"][0]["school"]["name"], "Santa Monica College");

    // Once a classmate is linked, the school is locked.
    let (mut b, _) = app.signup("b@stu.example.org").await;
    let res = b.post("/api/auth/school", json!({ "school_id": app.ids.uhs })).await;
    assert_eq!(res.json["user"]["school"]["name"], "Santa Monica College");
    let res = a.post("/api/auth/school", json!({ "school_id": app.ids.uhs })).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.code(), "school_locked");
    assert_eq!(
        res.json["error"]["message"],
        "Other students from this email domain already picked Santa Monica College. Contact us if that's wrong."
    );

    // An approved domain's school can't be re-picked.
    let (mut u, _) = app.signup("u@ucla.edu").await;
    let res = u.post("/api/auth/school", json!({ "school_id": app.ids.smc })).await;
    assert_eq!(res.code(), "forbidden");

    // A rejected student who is alone on the domain can re-pick; the request goes back to pending.
    let (mut r, _) = app.signup("r@solo.example.org").await;
    r.post("/api/auth/school", json!({ "school_id": app.ids.uhs })).await;
    let code = app.code_for("r@solo.example.org");
    r.post("/api/auth/verify", json!({ "code": code })).await;
    admin.post("/api/admin/domains/solo.example.org/reject", json!({ "reason": "Unknown school" })).await;
    assert_eq!(r.get("/api/me").await.json["user"]["status"], "rejected");
    let res = r.post("/api/auth/school", json!({ "school_id": app.ids.smc })).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["user"]["status"], "pending_school");
    assert_eq!(res.json["user"]["school"]["name"], "Santa Monica College");
    let domains = admin.get("/api/admin/domains?status=pending").await;
    assert!(domains.json["items"].as_array().unwrap().iter().any(|d| d["domain"] == "solo.example.org"));

    // Verified users can't re-pick.
    let mut v = app.verified("v@ucla.edu").await;
    assert_eq!(v.post("/api/auth/school", json!({ "school_id": app.ids.smc })).await.code(), "forbidden");
}
