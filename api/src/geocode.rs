use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum GeocodeError {
    #[error("geocoder request failed: {0}")]
    Http(String),
}

#[async_trait]
pub trait Geocoder: Send + Sync {
    /// Returns `(lat, lng)` for a free-form query, or `None` when nothing matches.
    async fn geocode(&self, query: &str) -> Result<Option<(f64, f64)>, GeocodeError>;
}

pub type DynGeocoder = Arc<dyn Geocoder>;

/// OSM Nominatim. Their usage policy allows at most one request per second and requires a User-Agent.
pub struct NominatimGeocoder {
    client: reqwest::Client,
    base_url: String,
    last_request: Mutex<Option<Instant>>,
}

impl NominatimGeocoder {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Corner/0.1 dev")
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            last_request: Mutex::new(None),
        }
    }
}

#[derive(Deserialize)]
struct NominatimHit {
    lat: String,
    lon: String,
}

#[async_trait]
impl Geocoder for NominatimGeocoder {
    async fn geocode(&self, query: &str) -> Result<Option<(f64, f64)>, GeocodeError> {
        {
            // Holding the lock across the sleep serializes callers, which is what enforces 1 req/s.
            let mut last = self.last_request.lock().await;
            if let Some(prev) = *last {
                let elapsed = prev.elapsed();
                if elapsed < Duration::from_secs(1) {
                    tokio::time::sleep(Duration::from_secs(1) - elapsed).await;
                }
            }
            *last = Some(Instant::now());
        }
        let hits: Vec<NominatimHit> = self
            .client
            .get(format!("{}/search", self.base_url))
            .query(&[("q", query), ("format", "jsonv2"), ("limit", "1")])
            .send()
            .await
            .map_err(|e| GeocodeError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| GeocodeError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| GeocodeError::Http(e.to_string()))?;
        Ok(hits.first().and_then(|h| {
            let lat = h.lat.parse().ok()?;
            let lng = h.lon.parse().ok()?;
            Some((lat, lng))
        }))
    }
}

/// Deterministic geocoder for tests: known queries map to coordinates, everything else uses `fallback`.
#[derive(Default)]
pub struct StubGeocoder {
    pub known: HashMap<String, (f64, f64)>,
    pub fallback: Option<(f64, f64)>,
}

#[async_trait]
impl Geocoder for StubGeocoder {
    async fn geocode(&self, query: &str) -> Result<Option<(f64, f64)>, GeocodeError> {
        Ok(self
            .known
            .get(&query.to_lowercase())
            .copied()
            .or(self.fallback))
    }
}
