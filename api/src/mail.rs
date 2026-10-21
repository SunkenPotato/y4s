use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("invalid address: {0}")]
    Address(String),
    #[error("smtp: {0}")]
    Smtp(String),
    #[error("resend: {0}")]
    Resend(String),
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, email: Email) -> Result<(), MailError>;
}

pub type DynMailer = Arc<dyn Mailer>;

/// Picks the transport from `MAIL_TRANSPORT`. Resend only activates when `RESEND_API_KEY` is set.
pub fn from_config(config: &Config) -> DynMailer {
    match config.mail_transport.as_str() {
        "resend" => match &config.resend_api_key {
            Some(key) => Arc::new(ResendMailer::new(key.clone(), config.mail_from.clone())),
            None => {
                tracing::warn!("MAIL_TRANSPORT=resend but RESEND_API_KEY is empty; logging emails instead");
                Arc::new(LogMailer)
            }
        },
        "log" => Arc::new(LogMailer),
        _ => Arc::new(SmtpMailer::new(&config.smtp_host, config.smtp_port, &config.mail_from)),
    }
}

pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl SmtpMailer {
    pub fn new(host: &str, port: u16, from: &str) -> Self {
        // Mailpit in dev speaks plain SMTP without TLS or auth.
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .build();
        Self {
            transport,
            from: from.to_string(),
        }
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, email: Email) -> Result<(), MailError> {
        let from: Mailbox = self.from.parse().map_err(|_| MailError::Address(self.from.clone()))?;
        let to: Mailbox = email.to.parse().map_err(|_| MailError::Address(email.to.clone()))?;
        let message = Message::builder()
            .from(from)
            .to(to)
            .subject(email.subject)
            .body(email.text)
            .map_err(|e| MailError::Smtp(e.to_string()))?;
        self.transport
            .send(message)
            .await
            .map_err(|e| MailError::Smtp(e.to_string()))?;
        Ok(())
    }
}

pub const RESEND_API_URL: &str = "https://api.resend.com";

pub struct ResendMailer {
    client: reqwest::Client,
    api_key: String,
    from: String,
    base_url: String,
}

impl ResendMailer {
    pub fn new(api_key: String, from: String) -> Self {
        Self::with_base_url(api_key, from, RESEND_API_URL)
    }

    /// Tests point this at a local stub so no real request is ever made.
    pub fn with_base_url(api_key: String, from: String, base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
            api_key,
            from,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl Mailer for ResendMailer {
    async fn send(&self, email: Email) -> Result<(), MailError> {
        let res = self
            .client
            .post(format!("{}/emails", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "from": self.from,
                "to": [email.to],
                "subject": email.subject,
                "text": email.text,
            }))
            .send()
            .await
            .map_err(|e| MailError::Resend(e.to_string()))?;
        if !res.status().is_success() {
            return Err(MailError::Resend(format!("status {}", res.status())));
        }
        Ok(())
    }
}

pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send(&self, email: Email) -> Result<(), MailError> {
        tracing::info!(to = %email.to, subject = %email.subject, body = %email.text, "email (log transport)");
        Ok(())
    }
}

/// Captures emails in memory; used by tests to read verification codes. `set_failing(true)`
/// makes every send fail, like a provider outage.
#[derive(Default, Clone)]
pub struct MemoryMailer {
    pub sent: Arc<Mutex<Vec<Email>>>,
    failing: Arc<std::sync::atomic::AtomicBool>,
}

impl MemoryMailer {
    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn messages(&self) -> Vec<Email> {
        self.sent.lock().map(|v| v.clone()).unwrap_or_default()
    }

    pub fn last_to(&self, to: &str) -> Option<Email> {
        self.messages().into_iter().rev().find(|e| e.to == to)
    }
}

#[async_trait]
impl Mailer for MemoryMailer {
    async fn send(&self, email: Email) -> Result<(), MailError> {
        if self.failing.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(MailError::Smtp("simulated failure".into()));
        }
        if let Ok(mut v) = self.sent.lock() {
            v.push(email);
        }
        Ok(())
    }
}

/// Sends and logs failures instead of failing the request; the user can always ask for a resend.
pub async fn send_logged(mailer: &DynMailer, email: Email) {
    let to = email.to.clone();
    if let Err(err) = mailer.send(email).await {
        tracing::error!(%to, error = %err, "failed to send email");
    }
}

pub fn code_email(to: &str, first_name: &str, code: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: format!("Your Corner code: {code}"),
        text: format!(
            "Hi {first_name},\n\nYour Corner verification code is {code}. It expires in 15 minutes.\n\nIf you didn't sign up for Corner, you can ignore this email.\n"
        ),
    }
}

pub fn school_change_code_email(to: &str, first_name: &str, code: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: format!("Your Corner code: {code}"),
        text: format!(
            "Hi {first_name},\n\nEnter {code} on Corner to move your account to this school email. It expires in 15 minutes.\n\nIf you didn't ask to change schools on Corner, you can ignore this email.\n"
        ),
    }
}

pub fn approved_email(to: &str, first_name: &str, school: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: "You're in".to_string(),
        text: format!(
            "Hi {first_name},\n\nWe confirmed your email domain for {school}. You're verified on Corner and can start writing reviews.\n"
        ),
    }
}

pub fn rejected_email(to: &str, first_name: &str, reason: Option<&str>) -> Email {
    let reason = reason
        .filter(|r| !r.trim().is_empty())
        .map(|r| format!("\nReason: {r}\n"))
        .unwrap_or_default();
    Email {
        to: to.to_string(),
        subject: "We couldn't verify your school email".to_string(),
        text: format!(
            "Hi {first_name},\n\nWe couldn't confirm that your email domain belongs to a school, so your Corner account isn't verified.\n{reason}\nIf you have an email from your school, sign up with that address instead.\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use axum::Router;

    /// Serves a fake Resend API on localhost that answers every send with `status`.
    async fn stub(status: StatusCode) -> String {
        let app = Router::new().route(
            "/emails",
            post(move |headers: HeaderMap, body: String| async move {
                let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or_default();
                if auth != "Bearer test-key" || !body.contains("\"to\":[\"a@ucla.edu\"]") {
                    return StatusCode::BAD_REQUEST;
                }
                status
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        format!("http://{addr}")
    }

    fn email() -> Email {
        Email { to: "a@ucla.edu".into(), subject: "s".into(), text: "t".into() }
    }

    #[tokio::test]
    async fn resend_success_and_error_mapping() {
        let ok = ResendMailer::with_base_url("test-key".into(), "Corner <x@y.z>".into(), &stub(StatusCode::OK).await);
        assert!(ok.send(email()).await.is_ok());

        let rejected = ResendMailer::with_base_url("test-key".into(), "Corner <x@y.z>".into(), &stub(StatusCode::FORBIDDEN).await);
        let err = rejected.send(email()).await.unwrap_err();
        assert!(matches!(err, MailError::Resend(_)));
        let app_err: crate::error::AppError = err.into();
        assert_eq!(app_err.code, "mail_failed");
        assert_eq!(app_err.status, StatusCode::BAD_GATEWAY);

        // Unreachable provider also maps to mail_failed.
        let down = ResendMailer::with_base_url("test-key".into(), "Corner <x@y.z>".into(), "http://127.0.0.1:9");
        let app_err: crate::error::AppError = down.send(email()).await.unwrap_err().into();
        assert_eq!(app_err.code, "mail_failed");
    }
}
