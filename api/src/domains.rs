//! Email domain rules for student verification.

/// Exact personal-provider domains.
const PERSONAL_EXACT: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "live.com",
    "icloud.com",
    "me.com",
    "aol.com",
    "proton.me",
    "protonmail.com",
    "mail.com",
    "zoho.com",
];

/// Providers listed as `name.*` in the contract (any TLD, e.g. yahoo.co.uk, hotmail.fr).
const PERSONAL_ANY_TLD: &[&str] = &["yahoo", "hotmail", "gmx", "yandex"];

pub fn is_personal_provider(domain: &str) -> bool {
    let domain = domain.to_lowercase();
    if PERSONAL_EXACT.contains(&domain.as_str()) {
        return true;
    }
    let first = domain.split('.').next().unwrap_or_default();
    PERSONAL_ANY_TLD.contains(&first) && domain.contains('.')
}

pub fn is_edu(domain: &str) -> bool {
    domain.ends_with(".edu")
}

/// The key a domain is stored under in `school_domains`. US `.edu` names are only registered at the
/// second level, so `g.ucla.edu` and `ucla.edu` share the key `ucla.edu`. Other domains match exactly.
pub fn domain_key(domain: &str) -> String {
    let domain = domain.to_lowercase();
    if is_edu(&domain) {
        let labels: Vec<&str> = domain.split('.').collect();
        if labels.len() > 2 {
            return labels[labels.len() - 2..].join(".");
        }
    }
    domain
}

/// Normalizes and validates an email; returns `(email, domain)` on success.
pub fn parse_email(raw: &str) -> Option<(String, String)> {
    let email = raw.trim().to_lowercase();
    if email.len() > 254 || email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let (local, domain) = email.split_once('@')?;
    if local.is_empty() || domain.contains('@') || !domain.contains('.') {
        return None;
    }
    let labels_ok = domain.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    });
    if !labels_ok {
        return None;
    }
    let domain = domain.to_string();
    Some((email, domain))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal() {
        assert!(is_personal_provider("gmail.com"));
        assert!(is_personal_provider("yahoo.co.uk"));
        assert!(is_personal_provider("hotmail.fr"));
        assert!(!is_personal_provider("ucla.edu"));
        assert!(!is_personal_provider("yahoostudents.org"));
    }

    #[test]
    fn keys() {
        assert_eq!(domain_key("g.ucla.edu"), "ucla.edu");
        assert_eq!(domain_key("ucla.edu"), "ucla.edu");
        assert_eq!(domain_key("students.lausd.net"), "students.lausd.net");
    }

    #[test]
    fn emails() {
        assert_eq!(
            parse_email(" Maya.R@G.UCLA.edu "),
            Some(("maya.r@g.ucla.edu".into(), "g.ucla.edu".into()))
        );
        assert_eq!(parse_email("nope"), None);
        assert_eq!(parse_email("a@b"), None);
        assert_eq!(parse_email("a b@c.edu"), None);
    }
}
