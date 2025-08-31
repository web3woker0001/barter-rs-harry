use crate::{PositionSide, RiskManager, TradingSignal};
use monitor_core::TradingConfig;

pub struct SimpleRiskManager {
    config: TradingConfig,
}

impl SimpleRiskManager {
    pub fn new(config: TradingConfig) -> Self {
        Self { config }
    }
}

impl RiskManager for SimpleRiskManager {
    fn validate_order(&self, signal: &TradingSignal, portfolio_value: f64) -> bool {
        // Check if position size is within limits
        let position_value = signal.price * self.calculate_position_size(signal, portfolio_value);
        
        if position_value > self.config.max_position_size {
            return false;
        }
        
        // Check risk percentage
        let risk_amount = portfolio_value * (self.config.risk_percentage / 100.0);
        if position_value > risk_amount {
            return false;
        }
        
        true
    }
    
    fn calculate_position_size(&self, signal: &TradingSignal, portfolio_value: f64) -> f64 {
        let risk_amount = portfolio_value * (self.config.risk_percentage / 100.0);
        let stop_loss_distance = signal.price * (self.config.stop_loss_percentage / 100.0);
        
        if stop_loss_distance > 0.0 {
            let position_size = risk_amount / stop_loss_distance;
            position_size.min(self.config.max_position_size / signal.price)
        } else {
            0.0
        }
    }
    
    fn get_stop_loss(&self, entry_price: f64, side: PositionSide) -> f64 {
        match side {
            PositionSide::Long => {
                entry_price * (1.0 - self.config.stop_loss_percentage / 100.0)
            }
            PositionSide::Short => {
                entry_price * (1.0 + self.config.stop_loss_percentage / 100.0)
            }
        }
    }
    
    fn get_take_profit(&self, entry_price: f64, side: PositionSide) -> f64 {
        match side {
            PositionSide::Long => {
                entry_price * (1.0 + self.config.take_profit_percentage / 100.0)
            }
            PositionSide::Short => {
                entry_price * (1.0 - self.config.take_profit_percentage / 100.0)
            }
        }
    }
}