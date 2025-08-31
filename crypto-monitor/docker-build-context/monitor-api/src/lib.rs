pub mod handlers;
pub mod websocket;
pub mod server;
pub mod state;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use monitor_core::{MonitorError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketDataQuery {
    pub symbol: Option<String>,
    pub exchange: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyQuery {
    pub symbol: Option<String>,
    pub exchange: Option<String>,
    pub anomaly_type: Option<String>,
    pub severity: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingConfig {
    pub enabled: bool,
    pub symbol: String,
    pub exchange: String,
    pub max_position_size: f64,
    pub stop_loss_percentage: f64,
    pub take_profit_percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub channels: Vec<AlertChannel>,
    pub severity_threshold: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertChannel {
    pub channel_type: ChannelType,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ChannelType {
    Telegram,
    WeChat,
    Email,
    SMS,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketStats {
    pub symbol: String,
    pub exchange: String,
    pub current_price: f64,
    pub volume_24h: f64,
    pub price_change_24h: f64,
    pub price_change_percentage_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub status: String,
    pub uptime_seconds: i64,
    pub connected_exchanges: Vec<ExchangeStatus>,
    pub active_monitors: i32,
    pub anomalies_detected_24h: i64,
    pub trades_executed_24h: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeStatus {
    pub name: String,
    pub connected: bool,
    pub last_heartbeat: DateTime<Utc>,
    pub active_symbols: Vec<String>,
}

pub type ApiResult<T> = std::result::Result<Json<ApiResponse<T>>, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ApiResponse::<()>::error(self.message));
        (self.status, body).into_response()
    }
}

impl From<MonitorError> for ApiError {
    fn from(err: MonitorError) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}