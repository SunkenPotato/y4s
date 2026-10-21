pub mod auth;
pub mod config;
pub mod db;
pub mod domains;
pub mod error;
pub mod extract;
pub mod geocode;
pub mod mail;
pub mod models;
pub mod pagination;
pub mod queries;
pub mod rate_limit;
pub mod routes;

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::Router;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::error::AppError;
use crate::geocode::DynGeocoder;
use crate::mail::DynMailer;
use crate::rate_limit::RateLimiter;
use crate::routes::{admin, auth as auth_routes, feed, me, places, reports, reviews, schools};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub mailer: DynMailer,
    pub geocoder: DynGeocoder,
    pub limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, mailer: DynMailer, geocoder: DynGeocoder) -> Self {
        Self {
            db,
            config: Arc::new(config),
            mailer,
            geocoder,
            limiter: Arc::new(RateLimiter::default()),
        }
    }
}

pub fn app(state: AppState) -> Router {
    let auth = Router::new()
        .route("/signup", post(auth_routes::signup))
        .route("/school", post(auth_routes::choose_school))
        .route("/verify", post(auth_routes::verify))
        .route("/resend", post(auth_routes::resend))
        .route("/login", post(auth_routes::login))
        .route("/logout", post(auth_routes::logout))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit::auth_limit));

    let admin = Router::new()
        .route("/domains", get(admin::list_domains))
        .route("/domains/{domain}/approve", post(admin::approve_domain))
        .route("/domains/{domain}/reject", post(admin::reject_domain))
        .route("/domains/{domain}/reset", post(admin::reset_domain))
        .route("/places", get(admin::list_places))
        .route("/places/{id}/approve", post(admin::approve_place))
        .route("/places/{id}/hide", post(admin::hide_place))
        .route("/reports", get(admin::list_reports))
        .route("/reports/{id}/resolve", post(admin::resolve_report));

    let api = Router::new()
        .nest("/auth", auth)
        .nest("/admin", admin)
        .route("/me", get(me::get_me).patch(me::patch_me))
        .route("/schools", get(schools::list))
        .route("/feed", get(feed::get_feed))
        .route("/places", get(places::list).post(places::create))
        .route("/places/{id}", get(places::get_place))
        .route("/places/{id}/reviews", get(reviews::list).post(reviews::create))
        .route("/reviews/{id}", axum::routing::patch(reviews::update).delete(reviews::delete))
        .route("/reviews/{id}/restore", post(reviews::restore))
        .route("/reviews/{id}/vote", put(reviews::vote).delete(reviews::unvote))
        .route("/reports", post(reports::create));

    let origins: Vec<HeaderValue> = state
        .config
        .allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .nest("/api", api)
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(middleware::from_fn_with_state(state.clone(), csrf))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn not_found() -> AppError {
    AppError::not_found()
}

async fn method_not_allowed() -> AppError {
    AppError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "method_not_allowed",
        "This endpoint doesn't support that method.",
    )
}

/// CSRF guard: every non-GET request needs a JSON content type, and a present `Origin` must be allowed.
async fn csrf(State(state): State<AppState>, req: Request, next: Next) -> Result<Response, AppError> {
    if !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        let is_json = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
        let origin_ok = match req.headers().get(header::ORIGIN) {
            None => true,
            Some(origin) => origin
                .to_str()
                .map(|o| state.config.allowed_origins.iter().any(|a| a.eq_ignore_ascii_case(o)))
                .unwrap_or(false),
        };
        if !is_json || !origin_ok {
            return Err(AppError::forbidden_origin());
        }
    }
    Ok(next.run(req).await)
}
