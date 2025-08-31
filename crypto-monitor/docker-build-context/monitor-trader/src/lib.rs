pub mod executor;
pub mod strategy;
pub mod risk;

use barter_execution::{
    order::{Order, OrderId, OrderKind, OrderState, OrderType},
    trade::Trade,
};
use barter_instrument::InstrumentIndex;
use chrono::{DateTime, Utc};
use monitor_anomaly::AnomalyDetection;
use monitor_core::{MonitorError, Result, TradingConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub id: uuid::Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub exchange: String,
    pub signal_type: SignalType,
    pub strength: SignalStrength,
    pub price: f64,
    pub reason: String,
    pub anomaly_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalType {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalStrength {
    Weak,
    Medium,
    Strong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: uuid::Uuid,
    pub symbol: String,
    pub exchange: String,
    pub side: PositionSide,
    pub quantity: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

impl Position {
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
        self.unrealized_pnl = match self.side {
            PositionSide::Long => (price - self.entry_price) * self.quantity,
            PositionSide::Short => (self.entry_price - price) * self.quantity,
        };
    }
    
    pub fn should_stop_loss(&self) -> bool {
        if let Some(stop_loss) = self.stop_loss {
            match self.side {
                PositionSide::Long => self.current_price <= stop_loss,
                PositionSide::Short => self.current_price >= stop_loss,
            }
        } else {
            false
        }
    }
    
    pub fn should_take_profit(&self) -> bool {
        if let Some(take_profit) = self.take_profit {
            match self.side {
                PositionSide::Long => self.current_price >= take_profit,
                PositionSide::Short => self.current_price <= take_profit,
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStats {
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub average_win: f64,
    pub average_loss: f64,
    pub profit_factor: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
}

impl Default for TradingStats {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            average_win: 0.0,
            average_loss: 0.0,
            profit_factor: 0.0,
            max_drawdown: 0.0,
            sharpe_ratio: 0.0,
        }
    }
}

pub trait TradingStrategy: Send + Sync {
    fn analyze(&mut self, anomaly: &AnomalyDetection) -> Option<TradingSignal>;
    fn update_config(&mut self, config: TradingConfig);
}

pub trait RiskManager: Send + Sync {
    fn validate_order(&self, signal: &TradingSignal, portfolio_value: f64) -> bool;
    fn calculate_position_size(&self, signal: &TradingSignal, portfolio_value: f64) -> f64;
    fn get_stop_loss(&self, entry_price: f64, side: PositionSide) -> f64;
    fn get_take_profit(&self, entry_price: f64, side: PositionSide) -> f64;
}