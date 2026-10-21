use std::env;

/// Runtime configuration. Every value has a dev default matching docs/api.md so `cargo run` works without a `.env`.
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub cookie_secure: bool,
    pub mail_transport: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub mail_from: String,
    pub resend_api_key: Option<String>,
    pub admin_emails: Vec<String>,
    pub nominatim_url: String,
    /// Requests per minute per IP on `/api/auth/*`.
    pub auth_rate_per_min: u32,
    /// Writes (reviews, places, reports, votes) per minute per user.
    pub write_rate_per_min: u32,
}

fn var(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: var("DATABASE_URL", "postgres://corner:corner@localhost:5433/corner"),
            port: var("PORT", "4000").parse().unwrap_or(4000),
            allowed_origins: list(&var("ALLOWED_ORIGINS", "http://localhost:3000")),
            cookie_secure: var("COOKIE_SECURE", "false") == "true",
            mail_transport: var("MAIL_TRANSPORT", "smtp").to_lowercase(),
            smtp_host: var("SMTP_HOST", "localhost"),
            smtp_port: var("SMTP_PORT", "1025").parse().unwrap_or(1025),
            mail_from: var("MAIL_FROM", "Corner <hello@corner.local>"),
            resend_api_key: env::var("RESEND_API_KEY").ok().filter(|v| !v.trim().is_empty()),
            admin_emails: list(&var("ADMIN_EMAILS", "admin@ucla.edu")),
            nominatim_url: var("NOMINATIM_URL", "https://nominatim.openstreetmap.org"),
            auth_rate_per_min: var("AUTH_RATE_PER_MIN", "10").parse().unwrap_or(10),
            write_rate_per_min: var("WRITE_RATE_PER_MIN", "30").parse().unwrap_or(30),
        }
    }

    /// Config for tests: no env lookups, generous rate limits.
    pub fn for_tests(database_url: &str) -> Self {
        Self {
            database_url: database_url.to_string(),
            port: 0,
            allowed_origins: vec!["http://localhost:3000".into()],
            cookie_secure: false,
            mail_transport: "log".into(),
            smtp_host: "localhost".into(),
            smtp_port: 1025,
            mail_from: "Corner <hello@corner.local>".into(),
            resend_api_key: None,
            admin_emails: vec!["admin@ucla.edu".into()],
            nominatim_url: "http://127.0.0.1:9".into(),
            auth_rate_per_min: 10_000,
            write_rate_per_min: 10_000,
        }
    }

    pub fn is_admin_email(&self, email: &str) -> bool {
        self.admin_emails.iter().any(|e| e == &email.to_lowercase())
    }
}
