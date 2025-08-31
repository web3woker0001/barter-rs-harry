pub mod engine;
pub mod event;
pub mod model;
pub mod storage;
pub mod stream;

use barter::EngineEvent;
use barter_data::event::MarketEvent;
use barter_execution::AccountEventKind;
use barter_instrument::InstrumentIndex;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("Fluvio error: {0}")]
    Fluvio(#[from] fluvio::FluvioError),
    
    #[error("Barter error: {0}")]
    Barter(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Stream error: {0}")]
    Stream(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MonitorError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorEvent {
    pub id: uuid::Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: EventSource,
    pub event_type: EventType,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    Exchange(String),
    Monitor,
    Anomaly,
    Trading,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    MarketData(MarketDataType),
    Anomaly(AnomalyType),
    Trade(TradeEventType),
    Alert(AlertType),
    System(SystemEventType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketDataType {
    Trade,
    OrderBook,
    Candle,
    Volume,
    Liquidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    VolumeSpike,
    PriceSpike,
    DepthImbalance,
    LargeOrder,
    UnusualActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeEventType {
    OrderPlaced,
    OrderFilled,
    OrderCancelled,
    PositionOpened,
    PositionClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEventType {
    Started,
    Stopped,
    Connected,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub exchanges: Vec<ExchangeConfig>,
    pub fluvio: FluvioConfig,
    pub database: DatabaseConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub enabled: bool,
    pub symbols: Vec<String>,
    pub subscriptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluvioConfig {
    pub endpoint: String,
    pub topic_prefix: String,
    pub partitions: u32,
    pub replication_factor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub anomaly_detection: AnomalyConfig,
    pub alerting: AlertConfig,
    pub trading: TradingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    pub volume_threshold_multiplier: f64,
    pub price_change_percentage: f64,
    pub lookback_window_minutes: u32,
    pub min_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub telegram_enabled: bool,
    pub wechat_enabled: bool,
    pub email_enabled: bool,
    pub sms_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub auto_trading_enabled: bool,
    pub max_position_size: f64,
    pub risk_percentage: f64,
    pub stop_loss_percentage: f64,
    pub take_profit_percentage: f64,
}