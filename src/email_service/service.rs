use lettre::{
    Message, SmtpTransport, Transport,
    message::builder::MessageBuilder,
    transport::smtp::authentication::Credentials,
};
use lettre::AsyncTokio1Transport; // Use the tokio-compatible transport
use std::env;
use tracing::{error, info};

/// The EmailService struct holds the mailer instance
#[derive(Clone)]
pub struct EmailService {
    mailer: SmtpTransport,
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
        let mailer = SmtpTransport::relay(&server)
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
    pub async fn send_email(
        &self,
        to: String,
        subject: String,
        body_text: String,
    ) {
        let email = Message::builder()
            .from(self.from_email.parse().unwrap())
            .to(to.parse().expect("Failed to parse 'to' email"))
            .subject(subject.clone())
            .body(body_text)
            .expect("Failed to build email");

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