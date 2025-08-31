use crate::{Notification, NotificationChannel, NotificationConfig};
use monitor_core::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct NotificationManager {
    channels: Arc<RwLock<Vec<Box<dyn NotificationChannel>>>>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn add_channel(&mut self, channel: Box<dyn NotificationChannel>) {
        let channels = self.channels.clone();
        tokio::spawn(async move {
            channels.write().await.push(channel);
        });
    }
    
    pub async fn send_all(&self, notification: &Notification) -> Result<()> {
        let channels = self.channels.read().await;
        
        for channel in channels.iter() {
            if channel.is_enabled() {
                info!("Sending notification via {}", channel.name());
                if let Err(e) = channel.send(notification).await {
                    error!("Failed to send via {}: {}", channel.name(), e);
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn send_to_channel(
        &self,
        channel_name: &str,
        notification: &Notification,
    ) -> Result<()> {
        let channels = self.channels.read().await;
        
        for channel in channels.iter() {
            if channel.name() == channel_name && channel.is_enabled() {
                return channel.send(notification).await;
            }
        }
        
        Err(monitor_core::MonitorError::Other(
            format!("Channel {} not found or disabled", channel_name)
        ))
    }
    
    pub async fn get_enabled_channels(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels
            .iter()
            .filter(|c| c.is_enabled())
            .map(|c| c.name().to_string())
            .collect()
    }
}