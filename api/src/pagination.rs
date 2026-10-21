use base64::Engine;
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// Validated `limit` and offset decoded from the opaque `cursor`.
#[derive(Clone, Copy, Debug)]
pub struct PageReq {
    pub limit: i64,
    pub offset: i64,
}

impl PageReq {
    pub fn parse(limit: Option<i64>, cursor: Option<&str>) -> AppResult<Self> {
        let limit = limit.unwrap_or(20);
        if !(1..=50).contains(&limit) {
            return Err(AppError::field("limit", "Use a limit from 1 to 50."));
        }
        let offset = match cursor.filter(|c| !c.is_empty()) {
            None => 0,
            Some(c) => decode_cursor(c).ok_or_else(|| AppError::field("cursor", "This cursor isn't valid."))?,
        };
        Ok(Self { limit, offset })
    }

    /// Rows are fetched with `limit + 1` so we know whether another page exists.
    pub fn fetch_limit(&self) -> i64 {
        self.limit + 1
    }

    pub fn finish<T>(&self, mut items: Vec<T>) -> Page<T> {
        let has_more = items.len() as i64 > self.limit;
        items.truncate(self.limit as usize);
        Page {
            next_cursor: has_more.then(|| encode_cursor(self.offset + self.limit)),
            items,
        }
    }
}

fn encode_cursor(offset: i64) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("o:{offset}"))
}

fn decode_cursor(cursor: &str) -> Option<i64> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(cursor).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let n: i64 = s.strip_prefix("o:")?.parse().ok()?;
    (n >= 0).then_some(n)
}
