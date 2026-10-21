mod common;

use axum::http::StatusCode;
use common::{TestApp, KNOWN_ADDRESS};
use serde_json::{json, Value};

const BODY: &str = "The spicy miso is worth the line. Go before six or you will wait half an hour.";

fn names(v: &Value) -> Vec<String> {
    v["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn places_search_filters_and_distance() {
    let app = TestApp::new().await;
    app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, Some("15% off Mon–Thu")).await;
    app.insert_place("Lindbrook Coffee", "coffee", 34.0598, -118.4440, Some("Free drip refill")).await;
    app.insert_place("Strathmore Tacos", "food", 34.0693, -118.4513, None).await;
    app.insert_place("Pico Taco Stand", "food", 34.0175, -118.4725, None).await; // ~3.5 mi away
    let c = app.client();

    assert_eq!(names(&c.get("/api/places?q=noodle").await.json), vec!["Gayley Noodle Bar"]);
    assert_eq!(names(&c.get("/api/places?q=gayl").await.json), vec!["Gayley Noodle Bar"]);
    assert_eq!(names(&c.get("/api/places?category=coffee").await.json), vec!["Lindbrook Coffee"]);
    let deals = names(&c.get("/api/places?deals=true").await.json);
    assert_eq!(deals.len(), 2);

    let near = c.get("/api/places?near=34.0689,-118.4452&sort=distance").await;
    assert_eq!(near.status, StatusCode::OK, "{}", near.json);
    let got = names(&near.json);
    assert_eq!(got, vec!["Strathmore Tacos", "Gayley Noodle Bar", "Lindbrook Coffee"]);
    let d: Vec<f64> = near.json["items"].as_array().unwrap().iter().map(|p| p["distance_mi"].as_f64().unwrap()).collect();
    assert!(d.windows(2).all(|w| w[0] <= w[1]), "{d:?}");
    assert!(d.iter().all(|x| *x <= 1.0));

    let wide = c.get("/api/places?near=34.0689,-118.4452&radius_mi=5&sort=distance&q=taco").await;
    assert_eq!(names(&wide.json), vec!["Strathmore Tacos", "Pico Taco Stand"]);

    // Without near there is no distance.
    assert!(c.get("/api/places").await.json["items"][0]["distance_mi"].is_null());

    let bad = c.get("/api/places?sort=distance").await;
    assert_eq!(bad.code(), "validation");
    assert!(bad.json["error"]["fields"]["sort"].is_string());
    assert_eq!(c.get("/api/places?category=bars").await.code(), "validation");
    assert_eq!(c.get("/api/places?radius_mi=50&near=34,-118").await.code(), "validation");

    // school_id: businesses within 2 mi of UCLA (Pico is ~3.5 mi away).
    let res = c.get(&format!("/api/places?school_id={}", app.ids.ucla)).await;
    assert_eq!(names(&res.json).len(), 3);

    let page = c.get("/api/places?limit=2").await;
    assert_eq!(page.json["items"].as_array().unwrap().len(), 2);
    assert!(page.json["next_cursor"].is_string());
}

#[tokio::test]
async fn feed_shape() {
    let app = TestApp::new().await;
    let place = app.insert_place("Gayley Noodle Bar", "food", 34.0632, -118.4469, Some("15% off Mon–Thu")).await;
    app.insert_place("Lindbrook Coffee", "coffee", 34.0598, -118.4440, None).await;
    let mut c = app.verified("maya@ucla.edu").await;
    c.post(&format!("/api/places/{place}/reviews"), json!({ "rating": 5, "body": BODY })).await;

    let feed = app.client().get("/api/feed").await;
    assert_eq!(feed.status, StatusCode::OK);
    let f = &feed.json;
    assert_eq!(f["school"]["name"], "UCLA");
    assert!(f["school"]["lat"].is_number());
    assert_eq!(f["featured"]["place"]["name"], "Gayley Noodle Bar");
    assert_eq!(f["featured"]["quote"]["text"], "The spicy miso is worth the line.");
    assert_eq!(f["featured"]["quote"]["author"]["display_name"], "Test S.");
    assert!(f["featured"]["quote"]["review_id"].is_string());
    assert_eq!(f["popular"][0]["name"], "Gayley Noodle Bar");
    assert_eq!(f["popular"].as_array().unwrap().len(), 2);
    assert_eq!(names(&json!({ "items": f["deals"] })), vec!["Gayley Noodle Bar"]);
    for key in [
        "id", "kind", "name", "category", "subcategory", "price_level", "address", "lat", "lng", "school_id",
        "student_discount", "rating_avg", "review_count", "distance_mi", "top_quote", "status",
    ] {
        assert!(f["popular"][0].get(key).is_some(), "Place is missing {key}");
    }

    let other = app.client().get(&format!("/api/feed?school_id={}", app.ids.smc)).await;
    assert_eq!(other.json["school"]["name"], "Santa Monica College");
    assert!(other.json["featured"].is_null());
    assert_eq!(other.json["popular"], json!([]));
    assert_eq!(app.client().get(&format!("/api/feed?school_id={}", uuid::Uuid::new_v4())).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_place_geocodes_dedupes_and_needs_approval() {
    let app = TestApp::new().await;
    let mut c = app.verified("adder@ucla.edu").await;
    let base = json!({
        "kind": "business", "name": "Tiverton Sushi Bar", "category": "food", "subcategory": "Sushi",
        "address": KNOWN_ADDRESS, "price_level": 2, "student_discount": "Free miso soup",
        "hours": [ { "days": "Mon – Thu", "time": "11 am – 10 pm" } ]
    });

    let res = c.post("/api/places", base.clone()).await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.json);
    let place = &res.json["place"];
    assert_eq!(place["status"], "pending");
    assert_eq!(place["lat"], 34.064);
    assert_eq!(place["hours"][0]["days"], "Mon – Thu");
    assert_eq!(place["rating_distribution"]["3"], 0);
    let id = place["id"].as_str().unwrap().to_string();

    // Pending: visible to the creator, 404 for everyone else.
    assert_eq!(c.get(&format!("/api/places/{id}")).await.status, StatusCode::OK);
    assert_eq!(app.client().get(&format!("/api/places/{id}")).await.status, StatusCode::NOT_FOUND);
    assert_eq!(names(&app.client().get("/api/places").await.json).len(), 0);

    let mut dup = base.clone();
    dup["name"] = json!("tiverton sushi-bar");
    let res = c.post("/api/places", dup).await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.code(), "duplicate_place");
    assert_eq!(res.json["error"]["existing_id"], id.as_str());

    let mut nowhere = base.clone();
    nowhere["name"] = json!("Somewhere Else");
    nowhere["address"] = json!("999 Nowhere Rd, Atlantis");
    let res = c.post("/api/places", nowhere).await;
    assert_eq!(res.code(), "geocode_failed");
    assert_eq!(res.json["error"]["fields"]["address"], "We couldn't find this address. Check the street and city.");

    let res = c.post("/api/places", json!({ "kind": "facility", "name": "Gym", "category": "gym", "address": KNOWN_ADDRESS })).await;
    assert!(res.json["error"]["fields"]["school_id"].is_string());

    let mut admin = app.admin().await;
    let queue = admin.get("/api/admin/places?status=pending").await;
    assert_eq!(queue.json["items"][0]["id"], id.as_str());
    assert_eq!(queue.json["items"][0]["created_by"]["display_name"], "Test S.");
    let res = admin.post(&format!("/api/admin/places/{id}/approve"), json!({})).await;
    assert_eq!(res.json["place"]["status"], "active");
    assert_eq!(names(&app.client().get("/api/places").await.json), vec!["Tiverton Sushi Bar"]);
    let res = admin.post(&format!("/api/admin/places/{id}/hide"), json!({})).await;
    assert_eq!(res.json["place"]["status"], "hidden");
    assert_eq!(app.client().get(&format!("/api/places/{id}")).await.status, StatusCode::NOT_FOUND);

    // Unverified users can't add places.
    let (mut fresh, _) = app.signup("fresh@ucla.edu").await;
    assert_eq!(fresh.post("/api/places", base).await.code(), "not_verified");
}

#[tokio::test]
async fn admin_endpoints_are_admin_only() {
    let app = TestApp::new().await;
    let mut user = app.verified("plain@ucla.edu").await;
    let mut anon = app.client();
    let gets = ["/api/admin/domains", "/api/admin/places", "/api/admin/reports"];
    for path in gets {
        let res = user.get(path).await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{path}");
        assert_eq!(res.code(), "forbidden");
        assert_eq!(anon.get(path).await.status, StatusCode::UNAUTHORIZED, "{path}");
    }
    let id = uuid::Uuid::new_v4();
    for path in [
        "/api/admin/domains/ucla.edu/approve".to_string(),
        "/api/admin/domains/ucla.edu/reject".to_string(),
        "/api/admin/domains/ucla.edu/reset".to_string(),
        format!("/api/admin/places/{id}/approve"),
        format!("/api/admin/places/{id}/hide"),
        format!("/api/admin/reports/{id}/resolve"),
    ] {
        let res = user.post(&path, json!({ "action": "dismiss" })).await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{path}");
        assert_eq!(anon.post(&path, json!({})).await.status, StatusCode::UNAUTHORIZED, "{path}");
    }
    let admin = app.admin().await;
    assert_eq!(admin.get("/api/admin/domains?status=approved").await.json["items"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn unknown_routes_use_error_shape() {
    let app = TestApp::new().await;
    let res = app.client().get("/api/nope").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.code(), "not_found");
    let res = app.client().get("/api/places/not-a-uuid").await;
    assert_eq!(res.code(), "not_found");
}
