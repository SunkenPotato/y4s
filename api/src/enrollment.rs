//! Enrollment-based access: a student's access lasts 5 years from the month they started.

use chrono::{Datelike, Months, NaiveDate, Utc};

use crate::error::{AppError, FieldErrors};
use crate::models::User;

pub const ACCESS_YEARS: u32 = 5;
pub const MAX_MONTHS_AHEAD: i32 = 12;

pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}

/// First day of `enrolled_on` + 5 years; posting stops on this date.
pub fn access_until(enrolled_on: NaiveDate) -> NaiveDate {
    enrolled_on
        .checked_add_months(Months::new(ACCESS_YEARS * 12))
        .unwrap_or(NaiveDate::MAX)
}

pub fn is_expired_on(enrolled_on: NaiveDate, today: NaiveDate) -> bool {
    access_until(enrolled_on) <= today
}

/// The status clients see. `expired` is never stored: it's a verified non-admin past `access_until`.
pub fn effective_status(user: &User) -> &str {
    if user.status == "verified" && user.role != "admin" && is_expired_on(user.enrolled_on, today()) {
        "expired"
    } else {
        &user.status
    }
}

pub fn format_month(d: NaiveDate) -> String {
    d.format("%Y-%m").to_string()
}

/// Parses `YYYY-MM` into the first of that month, at most 12 months ahead of `today`.
pub fn parse_enrolled_on(errors: &mut FieldErrors, raw: Option<&str>, today: NaiveDate) -> Option<NaiveDate> {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else {
        errors.add("enrolled_on", "Pick the month you started at this school.");
        return None;
    };
    let parsed = (raw.len() == 7)
        .then(|| NaiveDate::parse_from_str(&format!("{raw}-01"), "%Y-%m-%d").ok())
        .flatten();
    let Some(date) = parsed else {
        errors.add("enrolled_on", "Use the format YYYY-MM, like 2024-09.");
        return None;
    };
    let months_ahead = (date.year() - today.year()) * 12 + date.month() as i32 - today.month() as i32;
    if months_ahead > MAX_MONTHS_AHEAD {
        errors.add("enrolled_on", "Pick a start month no more than 12 months from now.");
        return None;
    }
    Some(date)
}

pub fn enrollment_too_old() -> AppError {
    AppError::unprocessable(
        "enrollment_too_old",
        "Corner is for current students. Access lasts 5 years from when you started.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn expiry_boundary() {
        let enrolled = d(2021, 10, 1);
        assert_eq!(access_until(enrolled), d(2026, 10, 1));
        assert!(!is_expired_on(enrolled, d(2026, 9, 30)));
        assert!(is_expired_on(enrolled, d(2026, 10, 1)));
    }

    #[test]
    fn parse_limits() {
        let today = d(2026, 10, 21);
        let mut e = FieldErrors::default();
        assert_eq!(parse_enrolled_on(&mut e, Some("2027-10"), today), Some(d(2027, 10, 1)));
        assert_eq!(parse_enrolled_on(&mut e, Some("2027-11"), today), None);
        assert_eq!(parse_enrolled_on(&mut e, Some("2024-9"), today), None);
        assert_eq!(parse_enrolled_on(&mut e, Some("2024-13"), today), None);
        assert_eq!(parse_enrolled_on(&mut e, None, today), None);
    }
}
