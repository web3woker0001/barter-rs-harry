use crate::{format_notification_message, Notification, NotificationChannel, TelegramConfig};
use async_trait::async_trait;
use monitor_core::{MonitorError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug)]
pub struct TelegramNotifier {
    config: TelegramConfig,
    client: Client,
}

impl TelegramNotifier {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
    
    async fn send_message(&self, chat_id: &str, text: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.config.bot_token
        );
        
        let params = SendMessageParams {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: Some("Markdown".to_string()),
            disable_web_page_preview: Some(true),
        };
        
        let response = self.client
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(|e| MonitorError::Other(format!("Telegram API error: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(MonitorError::Other(format!(
                "Telegram API returned error: {}",
                error_text
            )));
        }
        
        Ok(())
    }
}

#[async_trait]
impl NotificationChannel for TelegramNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        let message = format_notification_message(notification);
        
        for chat_id in &self.config.chat_ids {
            match self.send_message(chat_id, &message).await {
                Ok(_) => info!("Telegram notification sent to chat {}", chat_id),
                Err(e) => error!("Failed to send Telegram notification to {}: {}", chat_id, e),
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Telegram"
    }
    
    fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.chat_ids.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SendMessageParams {
    chat_id: String,
    text: String,
    parse_mode: Option<String>,
    disable_web_page_preview: Option<bool>,
}