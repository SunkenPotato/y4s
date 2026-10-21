use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::{AppError, AppResult};
use crate::AppState;

/// A small fixed-window limiter keyed by arbitrary strings ("auth:<ip>", "write:<user>").
#[derive(Default)]
pub struct RateLimiter {
    windows: Mutex<HashMap<String, (Instant, u32)>>,
}

const WINDOW: Duration = Duration::from_secs(60);

impl RateLimiter {
    /// Counts one hit for `key`. Returns the seconds to wait when `limit` per minute is exceeded.
    pub fn hit(&self, key: &str, limit: u32) -> Result<(), u64> {
        let now = Instant::now();
        let mut map = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() > 10_000 {
            map.retain(|_, (start, _)| now.duration_since(*start) < WINDOW);
        }
        let entry = map.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= WINDOW {
            *entry = (now, 0);
        }
        if entry.1 >= limit {
            let wait = WINDOW.saturating_sub(now.duration_since(entry.0));
            return Err(wait.as_secs().max(1));
        }
        entry.1 += 1;
        Ok(())
    }

    pub fn check(&self, key: &str, limit: u32) -> AppResult<()> {
        self.hit(key, limit).map_err(AppError::rate_limited)
    }
}

/// The client IP. Behind the Next.js rewrite the peer is the web server, so the last
/// `X-Forwarded-For` hop (the one our proxy appended) wins over the socket address.
pub fn client_ip(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| peer.map(|p| p.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// 10 requests per minute per IP on `/api/auth/*`.
pub async fn auth_limit(State(state): State<AppState>, req: Request, next: Next) -> Result<Response, AppError> {
    let peer = req.extensions().get::<ConnectInfo<SocketAddr>>().map(|c| c.0);
    let ip = client_ip(req.headers(), peer);
    state.limiter.check(&format!("auth:{ip}"), state.config.auth_rate_per_min)?;
    Ok(next.run(req).await)
}
