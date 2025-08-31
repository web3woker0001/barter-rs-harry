use crate::{EmailConfig, Notification, NotificationChannel};
use async_trait::async_trait;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use monitor_core::{MonitorError, Result};
use tracing::{error, info};

#[derive(Debug)]
pub struct EmailNotifier {
    config: EmailConfig,
    mailer: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl EmailNotifier {
    pub fn new(config: EmailConfig) -> Self {
        let mailer = if config.enabled {
            let creds = Credentials::new(
                config.username.clone(),
                config.password.clone(),
            );
            
            let builder = if config.use_tls {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            };
            
            builder
                .ok()
                .map(|b| b.credentials(creds).port(config.smtp_port).build())
        } else {
            None
        };
        
        Self { config, mailer }
    }
    
    fn build_email_body(&self, notification: &Notification) -> String {
        format!(
            r#"
            <html>
            <body>
                <h2>{}</h2>
                <p><strong>Alert Type:</strong> {:?}</p>
                <p><strong>Time:</strong> {}</p>
                <hr>
                <p>{}</p>
                {}
            </body>
            </html>
            "#,
            notification.title,
            notification.alert_type,
            notification.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            notification.message,
            if let Some(data) = &notification.data {
                format!("<pre>{}</pre>", serde_json::to_string_pretty(data).unwrap_or_default())
            } else {
                String::new()
            }
        )
    }
}

#[async_trait]
impl NotificationChannel for EmailNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        let mailer = self.mailer.as_ref().ok_or_else(|| {
            MonitorError::Other("Email mailer not initialized".to_string())
        })?;
        
        let body = self.build_email_body(notification);
        
        for to_address in &self.config.to_addresses {
            let email = Message::builder()
                .from(self.config.from_address.parse().map_err(|e| {
                    MonitorError::Other(format!("Invalid from address: {}", e))
                })?)
                .to(to_address.parse().map_err(|e| {
                    MonitorError::Other(format!("Invalid to address: {}", e))
                })?)
                .subject(&notification.title)
                .header(ContentType::TEXT_HTML)
                .body(body.clone())
                .map_err(|e| MonitorError::Other(format!("Failed to build email: {}", e)))?;
            
            match mailer.send(email).await {
                Ok(_) => info!("Email notification sent to {}", to_address),
                Err(e) => error!("Failed to send email to {}: {}", to_address, e),
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Email"
    }
    
    fn is_enabled(&self) -> bool {
        self.config.enabled && self.mailer.is_some() && !self.config.to_addresses.is_empty()
    }
}