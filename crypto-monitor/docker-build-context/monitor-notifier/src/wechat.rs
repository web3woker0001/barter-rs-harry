use crate::{format_notification_message, Notification, NotificationChannel, WeChatConfig};
use async_trait::async_trait;
use monitor_core::{MonitorError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug)]
pub struct WeChatNotifier {
    config: WeChatConfig,
    client: Client,
    access_token: Option<String>,
}

impl WeChatNotifier {
    pub fn new(config: WeChatConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            access_token: None,
        }
    }
    
    async fn get_access_token(&mut self) -> Result<String> {
        // Check if we have a valid token
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }
        
        // Get new token
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&corpsecret={}",
            self.config.corp_id, self.config.secret
        );
        
        let response: TokenResponse = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| MonitorError::Other(format!("WeChat API error: {}", e)))?
            .json()
            .await
            .map_err(|e| MonitorError::Other(format!("Failed to parse response: {}", e)))?;
        
        if response.errcode != 0 {
            return Err(MonitorError::Other(format!(
                "WeChat API error: {} - {}",
                response.errcode,
                response.errmsg.unwrap_or_default()
            )));
        }
        
        self.access_token = response.access_token.clone();
        Ok(response.access_token.unwrap_or_default())
    }
    
    async fn send_message(&mut self, message: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
            token
        );
        
        let msg = MessageRequest {
            touser: self.config.to_user.join("|"),
            msgtype: "text".to_string(),
            agentid: self.config.agent_id.parse().unwrap_or(0),
            text: TextContent {
                content: message.to_string(),
            },
            safe: 0,
        };
        
        let response: MessageResponse = self.client
            .post(&url)
            .json(&msg)
            .send()
            .await
            .map_err(|e| MonitorError::Other(format!("Failed to send message: {}", e)))?
            .json()
            .await
            .map_err(|e| MonitorError::Other(format!("Failed to parse response: {}", e)))?;
        
        if response.errcode != 0 {
            return Err(MonitorError::Other(format!(
                "Failed to send WeChat message: {} - {}",
                response.errcode,
                response.errmsg.unwrap_or_default()
            )));
        }
        
        Ok(())
    }
}

#[async_trait]
impl NotificationChannel for WeChatNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        let mut notifier = WeChatNotifier::new(self.config.clone());
        let message = format_notification_message(notification);
        
        match notifier.send_message(&message).await {
            Ok(_) => info!("WeChat notification sent"),
            Err(e) => error!("Failed to send WeChat notification: {}", e),
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "WeChat"
    }
    
    fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.to_user.is_empty()
    }
}

#[derive(Debug, Serialize)]
struct MessageRequest {
    touser: String,
    msgtype: String,
    agentid: i32,
    text: TextContent,
    safe: i32,
}

#[derive(Debug, Serialize)]
struct TextContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    errcode: i32,
    errmsg: Option<String>,
    access_token: Option<String>,
    expires_in: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    errcode: i32,
    errmsg: Option<String>,
    invaliduser: Option<String>,
}