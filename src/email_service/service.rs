use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use std::env;
use tracing::{error, info};

/// The EmailService struct holds the mailer instance
#[derive(Clone)]
pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
}

impl EmailService {
    /// Creates a new EmailService from environment variables
    pub fn new() -> Self {
        let server = env::var("SMTP_SERVER").expect("SMTP_SERVER must be set");
        let port = env::var("SMTP_PORT")
            .expect("SMTP_PORT must be set")
            .parse::<u16>()
            .expect("SMTP_PORT must be a valid number");
        let username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME must be set");
        let password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set");
        let from_email = env::var("SMTP_FROM_EMAIL").expect("SMTP_FROM_EMAIL must be set");

        let creds = Credentials::new(username, password);

        // Build the mailer
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&server)
            .expect("Failed to create SMTP relay")
            .port(port)
            .credentials(creds)
            .build();

        info!(smtp_server = %server, "Email service initialized");

        Self { mailer, from_email }
    }

    /// Sends an email asynchronously
    /// We use 'tokio::spawn' to send the email in the background
    /// so it doesn't block the API response.
    pub async fn send_email(&self, to: String, subject: String, body_text: String) {
        let Ok(from) = self.from_email.parse() else {
            error!(from_email = %self.from_email, "Failed to parse sender email");
            return;
        };

        let Ok(to_addr) = to.parse() else {
            error!(email_to = %to, "Failed to parse recipient email");
            return;
        };

        let Ok(email) = Message::builder()
            .from(from)
            .to(to_addr)
            .subject(subject.clone())
            .body(body_text)
        else {
            error!(email_to = %to, subject = %subject, "Failed to build email message");
            return;
        };

        let mailer = self.mailer.clone();

        // Spawn a background task for the email
        tokio::spawn(async move {
            match mailer.send(email).await {
                Ok(_) => info!(email_to = %to, subject = %subject, "Email sent successfully"),
                Err(e) => error!(email_to = %to, error = %e, "Failed to send email"),
            }
        });
    }
}
