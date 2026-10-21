use std::collections::BTreeMap;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{json, Map, Value};

/// Renders the contract's error envelope:
/// `{ "error": { "code", "message", "fields"?, ...extra } }`.
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub fields: Option<BTreeMap<String, String>>,
    pub extra: Map<String, Value>,
    pub retry_after: Option<u64>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            fields: None,
            extra: Map::new(),
            retry_after: None,
        }
    }

    /// Adds a top-level key inside the `error` object, e.g. `attempts_left` or `review_id`.
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.extra.insert(key.to_string(), value.into());
        self
    }

    pub fn with_fields(mut self, fields: BTreeMap<String, String>) -> Self {
        self.fields = Some(fields);
        self
    }

    pub fn unauthenticated() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthenticated", "Log in to continue.")
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn not_verified() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "not_verified",
            "Verify your student email to do this.",
        )
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "We couldn't find that.")
    }

    pub fn forbidden_origin() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden_origin",
            "Send requests from the Corner site with a JSON content type.",
        )
    }

    pub fn validation(fields: BTreeMap<String, String>) -> Self {
        let message = if fields.len() == 1 {
            fields.values().next().cloned().unwrap_or_default()
        } else {
            "Check the highlighted fields and try again.".to_string()
        };
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "validation", message).with_fields(fields)
    }

    pub fn field(field: &str, message: impl Into<String>) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert(field.to_string(), message.into());
        Self::validation(fields)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn unprocessable(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        let mut e = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            format!("Too many requests. Try again in {retry_after} seconds."),
        )
        .with("retry_after", retry_after);
        e.retry_after = Some(retry_after);
        e
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Something went wrong on our side. Try again in a moment.",
        )
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut body = Map::new();
        body.insert("code".into(), json!(self.code));
        body.insert("message".into(), json!(self.message));
        if let Some(fields) = &self.fields {
            body.insert("fields".into(), json!(fields));
        }
        for (k, v) in self.extra {
            body.insert(k, v);
        }
        let mut res = (self.status, axum::Json(json!({ "error": body }))).into_response();
        if let Some(secs) = self.retry_after {
            if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
                res.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        res
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        if matches!(err, sqlx::Error::RowNotFound) {
            return Self::not_found();
        }
        tracing::error!(error = %err, "database error");
        Self::internal()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!(error = %err, "internal error");
        Self::internal()
    }
}

impl From<JsonRejection> for AppError {
    fn from(rej: JsonRejection) -> Self {
        match rej {
            JsonRejection::MissingJsonContentType(_) => Self::forbidden_origin(),
            JsonRejection::JsonDataError(e) => {
                // serde's message names the offending field, which is useful to a developer.
                Self::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    format!("Check the request body: {}", e.body_text()),
                )
                .with_fields(BTreeMap::new())
            }
            _ => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
                "Send a valid JSON body.",
            )
            .with_fields(BTreeMap::new()),
        }
    }
}

impl From<QueryRejection> for AppError {
    fn from(rej: QueryRejection) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation",
            format!("Check the query parameters: {}", rej.body_text()),
        )
        .with_fields(BTreeMap::new())
    }
}

impl From<PathRejection> for AppError {
    fn from(_: PathRejection) -> Self {
        Self::not_found()
    }
}

/// Collects field errors and turns them into a single `validation` error.
#[derive(Default)]
pub struct FieldErrors(pub BTreeMap<String, String>);

impl FieldErrors {
    pub fn add(&mut self, field: &str, message: impl Into<String>) {
        self.0.entry(field.to_string()).or_insert_with(|| message.into());
    }

    pub fn check(self) -> AppResult<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(AppError::validation(self.0))
        }
    }
}
