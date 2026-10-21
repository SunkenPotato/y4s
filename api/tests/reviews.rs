mod common;

use axum::http::{Method, StatusCode};
use common::TestApp;
use serde_json::json;

const BODY: &str = "The spicy miso is worth the line. Go before six or you will wait half an hour.";

#[tokio::test]
async fn unverified_user_cannot_review() {
    let app = TestApp::new().await;
    let place = app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, None).await;
    let (mut c, _) = app.signup("new@ucla.edu").await;
    let res = c.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 5, "body": BODY })).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.code(), "not_verified");

    let mut anon = app.client();
    let res = anon.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 5, "body": BODY })).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.code(), "unauthenticated");
}

#[tokio::test]
async fn create_validate_and_duplicate_review() {
    let app = TestApp::new().await;
    let place = app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, Some("15% off Mon–Thu")).await;
    let mut c = app.verified("maya@ucla.edu").await;
    let path = format!("/api/places/{place}/reviews");

    let res = c.post(&path, json!({ "rating": 0, "body": "too short" })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["fields"]["rating"], "Pick a rating from 1 to 5 stars.");
    assert_eq!(res.json["error"]["fields"]["body"], "Write at least 50 characters so it's useful.");

    let res = c.post(&path, json!({ "rating": 5, "body": BODY, "visited_month": "2099-01" })).await;
    assert!(res.json["error"]["fields"]["visited_month"].is_string());

    let res = c
        .post(&path, json!({ "rating": 5, "body": BODY, "visited_month": "2026-01", "used_deal": true }))
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
    let review = &res.json["review"];
    assert_eq!(review["rating"], 5);
    assert_eq!(review["visited_month"], "2026-01");
    assert_eq!(review["used_deal"], true);
    assert_eq!(review["helpful_count"], 0);
    assert_eq!(review["viewer_voted"], false);
    assert_eq!(review["is_mine"], true);
    assert_eq!(review["author"]["display_name"], "Test S.");
    assert_eq!(review["author"]["school_name"], "UCLA");
    let review_id = review["id"].as_str().unwrap().to_string();

    let res = c.post(&path, json!({ "rating": 4, "body": BODY })).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.code(), "already_reviewed");
    assert_eq!(res.json["error"]["review_id"], review_id.as_str());

    // Place detail reflects the review.
    let detail = c.get(&format!("/api/places/{place}")).await.json;
    assert_eq!(detail["place"]["review_count"], 1);
    assert_eq!(detail["place"]["rating_avg"], 5.0);
    assert_eq!(detail["place"]["rating_distribution"]["5"], 1);
    assert_eq!(detail["place"]["rating_distribution"]["1"], 0);
    assert_eq!(detail["place"]["viewer_review_id"], review_id.as_str());
    assert_eq!(detail["place"]["top_quote"]["text"], "The spicy miso is worth the line.");

    // Edit: author only.
    let res = c
        .json(Method::PATCH, &format!("/api/reviews/{review_id}"), json!({ "rating": 4, "visited_month": null }))
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["review"]["rating"], 4);
    assert!(res.json["review"]["visited_month"].is_null());
    assert_eq!(res.json["review"]["body"], BODY);

    let mut other = app.verified("other@ucla.edu").await;
    let res = other.json(Method::PATCH, &format!("/api/reviews/{review_id}"), json!({ "rating": 1 })).await;
    assert_eq!(res.code(), "forbidden");

    let res = c.get(&format!("/api/places/{}/reviews", uuid::Uuid::new_v4())).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn votes_are_idempotent_and_not_on_own_review() {
    let app = TestApp::new().await;
    let place = app.insert_place("Lindbrook Coffee", "coffee", 34.0598, -118.4440, None).await;
    let mut author = app.verified("author@ucla.edu").await;
    let res = author.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 4, "body": BODY })).await;
    let id = res.json["review"]["id"].as_str().unwrap().to_string();
    let vote = format!("/api/reviews/{id}/vote");

    let res = author.json(Method::PUT, &vote, json!({})).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let mut voter = app.verified("voter@ucla.edu").await;
    for _ in 0..2 {
        let res = voter.json(Method::PUT, &vote, json!({})).await;
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.json, json!({ "helpful_count": 1, "viewer_voted": true }));
    }
    let list = voter.get(&format!("/api/places/{place}/reviews")).await;
    assert_eq!(list.json["items"][0]["viewer_voted"], true);
    assert_eq!(list.json["items"][0]["is_mine"], false);
    assert!(list.json["next_cursor"].is_null());

    for _ in 0..2 {
        let res = voter.json(Method::DELETE, &vote, json!({})).await;
        assert_eq!(res.json, json!({ "helpful_count": 0, "viewer_voted": false }));
    }
}

#[tokio::test]
async fn soft_delete_and_restore() {
    let app = TestApp::new().await;
    let place = app.insert_place("Strathmore Tacos", "food", 34.0693, -118.4513, None).await;
    let mut c = app.verified("del@ucla.edu").await;
    let res = c.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 3, "body": BODY })).await;
    let id = res.json["review"]["id"].as_str().unwrap().to_string();

    let mut other = app.verified("nosy@ucla.edu").await;
    let res = other.json(Method::DELETE, &format!("/api/reviews/{id}"), json!({})).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = c.json(Method::DELETE, &format!("/api/reviews/{id}"), json!({})).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let list = c.get(&format!("/api/places/{place}/reviews")).await;
    assert_eq!(list.json["items"].as_array().unwrap().len(), 0);
    assert!(c.get(&format!("/api/places/{place}")).await.json["place"]["viewer_review_id"].is_null());

    let res = c.post(&format!("/api/reviews/{id}/restore"), json!({})).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["review"]["id"], id.as_str());
    assert_eq!(c.get(&format!("/api/places/{place}/reviews")).await.json["items"].as_array().unwrap().len(), 1);

    // Outside the 5-minute window the delete sticks.
    c.json(Method::DELETE, &format!("/api/reviews/{id}"), json!({})).await;
    sqlx::query("UPDATE reviews SET deleted_at = now() - interval '6 minutes'")
        .execute(&app.pool)
        .await
        .unwrap();
    let res = c.post(&format!("/api/reviews/{id}/restore"), json!({})).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    // Deleted reviews no longer block a new one.
    let res = c.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 4, "body": BODY })).await;
    assert_eq!(res.status, StatusCode::CREATED);
}

#[tokio::test]
async fn reviews_sorting_and_pagination() {
    let app = TestApp::new().await;
    let place = app.insert_place("Weyburn Thai Kitchen", "food", 34.0629, -118.4452, None).await;
    for (i, rating) in [5, 2, 4].iter().enumerate() {
        let mut c = app.verified(&format!("s{i}@ucla.edu")).await;
        c.post(&format!("/api/places/{place}/reviews"), json!({ "rating": rating, "body": BODY })).await;
    }
    let c = app.client();
    let lowest = c.get(&format!("/api/places/{place}/reviews?sort=lowest")).await;
    assert_eq!(lowest.json["items"][0]["rating"], 2);
    let page1 = c.get(&format!("/api/places/{place}/reviews?sort=new&limit=2")).await;
    assert_eq!(page1.json["items"].as_array().unwrap().len(), 2);
    let cursor = page1.json["next_cursor"].as_str().unwrap();
    let page2 = c.get(&format!("/api/places/{place}/reviews?sort=new&limit=2&cursor={cursor}")).await;
    assert_eq!(page2.json["items"].as_array().unwrap().len(), 1);
    assert!(page2.json["next_cursor"].is_null());
    assert_eq!(c.get(&format!("/api/places/{place}/reviews?limit=51")).await.code(), "validation");
}

#[tokio::test]
async fn reports_dedupe_and_admin_resolve() {
    let app = TestApp::new().await;
    let place = app.insert_place("Broxton Karaoke Lounge", "nightlife", 34.0631, -118.4481, None).await;
    let mut author = app.verified("spammer@ucla.edu").await;
    let res = author.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 5, "body": BODY })).await;
    let review_id = res.json["review"]["id"].as_str().unwrap().to_string();

    let mut reporter = app.verified("reporter@ucla.edu").await;
    let body = json!({ "target_type": "review", "target_id": review_id, "reason": "spam", "details": "Looks like an ad." });
    let first = reporter.post("/api/reports", body.clone()).await;
    assert_eq!(first.status, StatusCode::CREATED);
    let again = reporter.post("/api/reports", body).await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(again.json["id"], first.json["id"]);
    let res = reporter
        .post("/api/reports", json!({ "target_type": "review", "target_id": review_id, "reason": "bogus" }))
        .await;
    assert_eq!(res.code(), "validation");

    let mut admin = app.admin().await;
    let list = admin.get("/api/admin/reports?status=open").await;
    let item = &list.json["items"][0];
    assert_eq!(item["reason"], "spam");
    assert_eq!(item["report_count"], 1);
    assert_eq!(item["reporter"]["display_name"], "Test S.");
    assert!(item["target_preview"].as_str().unwrap().starts_with("The spicy miso"));

    let id = first.json["id"].as_str().unwrap();
    let res = admin.post(&format!("/api/admin/reports/{id}/resolve"), json!({ "action": "remove_content" })).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    assert_eq!(res.json["report"]["status"], "resolved");
    let list = app.client().get(&format!("/api/places/{place}/reviews")).await;
    assert_eq!(list.json["items"].as_array().unwrap().len(), 0);
    // The author can't undo a moderator's removal.
    let res = author.post(&format!("/api/reviews/{review_id}/restore"), json!({})).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}
