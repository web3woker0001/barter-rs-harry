use crate::{Notification, NotificationChannel, SmsConfig, SmsProvider};
use async_trait::async_trait;
use monitor_core::{MonitorError, Result};
use reqwest::Client;
use tracing::{error, info};

#[derive(Debug)]
pub struct SmsNotifier {
    config: SmsConfig,
    client: Client,
}

impl SmsNotifier {
    pub fn new(config: SmsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    async fn send_twilio(&self, to: &str, message: &str) -> Result<()> {
        if let SmsProvider::Twilio { account_sid, auth_token } = &self.config.provider {
            let url = format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
                account_sid
            );
            
            let params = [
                ("From", self.config.from_number.as_str()),
                ("To", to),
                ("Body", message),
            ];
            
            let response = self.client
                .post(&url)
                .basic_auth(account_sid, Some(auth_token))
                .form(&params)
                .send()
                .await
                .map_err(|e| MonitorError::Other(format!("Twilio API error: {}", e)))?;
            
            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(MonitorError::Other(format!("Twilio error: {}", error_text)));
            }
            
            Ok(())
        } else {
            Err(MonitorError::Other("Invalid SMS provider configuration".to_string()))
        }
    }
    
    async fn send_aliyun(&self, to: &str, message: &str) -> Result<()> {
        // Aliyun SMS implementation would go here
        // This is a placeholder
        Err(MonitorError::Other("Aliyun SMS not implemented yet".to_string()))
    }
}

#[async_trait]
impl NotificationChannel for SmsNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        let message = format!(
            "{}: {}",
            notification.title,
            notification.message
        );
        
        for number in &self.config.to_numbers {
            let result = match &self.config.provider {
                SmsProvider::Twilio { .. } => self.send_twilio(number, &message).await,
                SmsProvider::Aliyun { .. } => self.send_aliyun(number, &message).await,
            };
            
            match result {
                Ok(_) => info!("SMS sent to {}", number),
                Err(e) => error!("Failed to send SMS to {}: {}", number, e),
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "SMS"
    }
    
    fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.to_numbers.is_empty()
    }
}