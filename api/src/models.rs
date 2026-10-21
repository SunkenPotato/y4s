//! Database rows and the contract's shared JSON objects.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

pub const CATEGORIES: &[&str] = &[
    "food", "coffee", "study", "gym", "services", "nightlife", "groceries", "other",
];

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub email_domain: String,
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
    pub birth_year: i32,
    pub role: String,
    pub status: String,
    pub school_id: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub domain_decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// "Maya R." — first name plus last initial.
pub fn display_name(first: &str, last: &str) -> String {
    match last.trim().chars().next() {
        Some(c) => format!("{} {}.", first.trim(), c.to_uppercase()),
        None => first.trim().to_string(),
    }
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct SchoolSummary {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub city: String,
    pub state: String,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct School {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub city: String,
    pub state: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

impl School {
    pub fn summary(&self) -> SchoolSummary {
        SchoolSummary {
            id: self.id,
            name: self.name.clone(),
            kind: self.kind.clone(),
            city: self.city.clone(),
            state: self.state.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Me {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub school: Option<SchoolSummary>,
    pub needs_school: bool,
    pub created_at: DateTime<Utc>,
}

impl Me {
    pub fn new(user: &User, school: Option<SchoolSummary>) -> Self {
        Self {
            id: user.id,
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            display_name: display_name(&user.first_name, &user.last_name),
            role: user.role.clone(),
            status: user.status.clone(),
            needs_school: user.school_id.is_none(),
            school,
            created_at: user.created_at,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Author {
    pub display_name: String,
    pub school_name: Option<String>,
    pub school_kind: Option<String>,
}

impl Author {
    pub fn new(first: &str, last: &str, school_name: Option<String>, school_kind: Option<String>) -> Self {
        Self {
            display_name: display_name(first, last),
            school_name,
            school_kind,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Quote {
    pub text: String,
    pub author: Author,
    pub review_id: Uuid,
}

/// A place row with computed stats; `distance_mi` is null unless the query had a reference point.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PlaceRow {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub price_level: Option<i16>,
    pub address: String,
    pub lat: f64,
    pub lng: f64,
    pub school_id: Option<Uuid>,
    pub student_discount: Option<String>,
    pub status: String,
    pub hours: Option<serde_json::Value>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub rating_avg: Option<f64>,
    pub review_count: i64,
    pub distance_mi: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Place {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub price_level: Option<i16>,
    pub address: String,
    pub lat: f64,
    pub lng: f64,
    pub school_id: Option<Uuid>,
    pub student_discount: Option<String>,
    pub rating_avg: Option<f64>,
    pub review_count: i64,
    pub distance_mi: Option<f64>,
    pub top_quote: Option<Quote>,
    pub status: String,
}

impl Place {
    pub fn from_row(row: &PlaceRow, top_quote: Option<Quote>) -> Self {
        Self {
            id: row.id,
            kind: row.kind.clone(),
            name: row.name.clone(),
            category: row.category.clone(),
            subcategory: row.subcategory.clone(),
            price_level: row.price_level,
            address: row.address.clone(),
            lat: row.lat,
            lng: row.lng,
            school_id: row.school_id,
            student_discount: row.student_discount.clone(),
            rating_avg: row.rating_avg,
            review_count: row.review_count,
            distance_mi: row.distance_mi,
            top_quote,
            status: row.status.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PlaceDetail {
    #[serde(flatten)]
    pub place: Place,
    pub hours: Option<serde_json::Value>,
    pub rating_distribution: std::collections::BTreeMap<String, i64>,
    pub school: Option<SchoolSummary>,
    pub viewer_review_id: Option<Uuid>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ReviewRow {
    pub id: Uuid,
    pub place_id: Uuid,
    pub user_id: Uuid,
    pub rating: i16,
    pub body: String,
    pub visited_month: Option<NaiveDate>,
    pub used_deal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub helpful_count: i64,
    pub viewer_voted: bool,
    pub first_name: String,
    pub last_name: String,
    pub school_name: Option<String>,
    pub school_kind: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Review {
    pub id: Uuid,
    pub place_id: Uuid,
    pub rating: i16,
    pub body: String,
    pub visited_month: Option<String>,
    pub used_deal: bool,
    pub helpful_count: i64,
    pub viewer_voted: bool,
    pub is_mine: bool,
    pub author: Author,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Review {
    pub fn from_row(row: &ReviewRow, viewer: Option<Uuid>) -> Self {
        Self {
            id: row.id,
            place_id: row.place_id,
            rating: row.rating,
            body: row.body.clone(),
            visited_month: row.visited_month.map(|d| d.format("%Y-%m").to_string()),
            used_deal: row.used_deal,
            helpful_count: row.helpful_count,
            viewer_voted: row.viewer_voted,
            is_mine: viewer == Some(row.user_id),
            author: Author::new(
                &row.first_name,
                &row.last_name,
                row.school_name.clone(),
                row.school_kind.clone(),
            ),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// The first sentence of `text`, at most `max` chars, ending at a sentence boundary when possible.
pub fn first_sentence(text: &str, max: usize) -> String {
    let text = text.trim();
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i >= max {
            break;
        }
        if matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace()) {
            return chars[..=i].iter().collect();
        }
    }
    if chars.len() <= max {
        return text.to_string();
    }
    // No boundary in range: cut at the last word break and add an ellipsis.
    let cut: String = chars[..max - 1].iter().collect();
    let cut = match cut.rfind(char::is_whitespace) {
        Some(idx) if idx > max / 2 => cut[..idx].to_string(),
        _ => cut,
    };
    format!("{}…", cut.trim_end_matches(|c: char| c.is_whitespace() || c == ','))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names() {
        assert_eq!(display_name("Maya", "Rivera"), "Maya R.");
        assert_eq!(display_name("Li", ""), "Li");
    }

    #[test]
    fn first_sentences() {
        assert_eq!(
            first_sentence("The spicy miso is worth the line. Go before six.", 140),
            "The spicy miso is worth the line."
        );
        assert_eq!(first_sentence("Costs $2.50 a cup! Nice.", 140), "Costs $2.50 a cup!");
        let long = "word ".repeat(60);
        let s = first_sentence(&long, 140);
        assert!(s.chars().count() <= 140);
        assert!(s.ends_with('…'));
    }
}
