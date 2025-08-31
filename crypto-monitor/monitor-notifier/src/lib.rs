pub mod telegram;
pub mod wechat;
pub mod email;
pub mod sms;
pub mod manager;

use async_trait::async_trait;
use monitor_anomaly::AnomalyDetection;
use monitor_core::{AlertType, MonitorError, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub alert_type: AlertType,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl Notification {
    pub fn from_anomaly(anomaly: &AnomalyDetection) -> Self {
        let alert_type = match anomaly.severity {
            monitor_anomaly::AnomalySeverity::Critical => AlertType::Critical,
            monitor_anomaly::AnomalySeverity::High => AlertType::Warning,
            _ => AlertType::Info,
        };
        
        Self {
            id: uuid::Uuid::new_v4(),
            timestamp: anomaly.timestamp,
            alert_type,
            title: format!("{:?} detected on {}/{}", 
                anomaly.anomaly_type, 
                anomaly.exchange, 
                anomaly.symbol
            ),
            message: anomaly.description.clone(),
            data: Some(serde_json::to_value(anomaly).unwrap_or_default()),
        }
    }
}

#[async_trait]
pub trait NotificationChannel: Send + Sync + Debug {
    async fn send(&self, notification: &Notification) -> Result<()>;
    fn name(&self) -> &str;
    fn is_enabled(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub telegram: TelegramConfig,
    pub wechat: WeChatConfig,
    pub email: EmailConfig,
    pub sms: SmsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_ids: Vec<String>,
    pub send_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatConfig {
    pub enabled: bool,
    pub corp_id: String,
    pub agent_id: String,
    pub secret: String,
    pub to_user: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub enabled: bool,
    pub provider: SmsProvider,
    pub from_number: String,
    pub to_numbers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmsProvider {
    Twilio {
        account_sid: String,
        auth_token: String,
    },
    Aliyun {
        access_key_id: String,
        access_key_secret: String,
        sign_name: String,
        template_code: String,
    },
}

pub fn format_notification_message(notification: &Notification) -> String {
    let emoji = match notification.alert_type {
        AlertType::Critical => "🚨",
        AlertType::Warning => "⚠️",
        AlertType::Info => "ℹ️",
    };
    
    format!(
        "{} *{}*\n\n{}\n\n_Time: {}_",
        emoji,
        notification.title,
        notification.message,
        notification.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    )
}