use crate::{
    Position, PositionSide, RiskManager, TradingSignal, TradingStats, TradingStrategy,
};
use barter_execution::{
    ExecutionClient,
    order::{Order, OrderId, OrderKind, OrderState, OrderType, RequestCancel, RequestOpen},
};
use dashmap::DashMap;
use monitor_anomaly::AnomalyDetection;
use monitor_core::{MonitorError, Result, TradingConfig};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct AutoTrader {
    config: Arc<RwLock<TradingConfig>>,
    strategy: Arc<RwLock<Box<dyn TradingStrategy>>>,
    risk_manager: Arc<Box<dyn RiskManager>>,
    execution_client: Arc<dyn ExecutionClient>,
    positions: Arc<DashMap<String, Position>>,
    stats: Arc<RwLock<TradingStats>>,
    portfolio_value: Arc<RwLock<f64>>,
}

impl AutoTrader {
    pub fn new(
        config: TradingConfig,
        strategy: Box<dyn TradingStrategy>,
        risk_manager: Box<dyn RiskManager>,
        execution_client: Arc<dyn ExecutionClient>,
        initial_portfolio: f64,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            strategy: Arc::new(RwLock::new(strategy)),
            risk_manager: Arc::new(risk_manager),
            execution_client,
            positions: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(TradingStats::default())),
            portfolio_value: Arc::new(RwLock::new(initial_portfolio)),
        }
    }
    
    pub async fn process_anomaly(&self, anomaly: &AnomalyDetection) -> Result<()> {
        if !self.config.read().auto_trading_enabled {
            return Ok(());
        }
        
        // Generate trading signal from anomaly
        let signal = {
            let mut strategy = self.strategy.write();
            strategy.analyze(anomaly)
        };
        
        if let Some(signal) = signal {
            info!("Trading signal generated: {:?}", signal);
            self.execute_signal(signal).await?;
        }
        
        Ok(())
    }
    
    async fn execute_signal(&self, signal: TradingSignal) -> Result<()> {
        let portfolio_value = *self.portfolio_value.read();
        
        // Validate order with risk manager
        if !self.risk_manager.validate_order(&signal, portfolio_value) {
            warn!("Order rejected by risk manager: {:?}", signal);
            return Ok(());
        }
        
        // Calculate position size
        let quantity = self.risk_manager.calculate_position_size(&signal, portfolio_value);
        
        // Determine order side
        let (side, position_side) = match signal.signal_type {
            crate::SignalType::Buy => (OrderKind::Buy, PositionSide::Long),
            crate::SignalType::Sell => (OrderKind::Sell, PositionSide::Short),
            crate::SignalType::Hold => return Ok(()),
        };
        
        // Create order request
        let order_request = RequestOpen {
            instrument: signal.symbol.clone(),
            exchange: signal.exchange.clone(),
            kind: side,
            order_type: OrderType::Market,
            quantity,
            price: Some(signal.price),
            time_in_force: None,
            post_only: false,
            reduce_only: false,
        };
        
        // Execute order
        match self.execution_client.open_order(order_request).await {
            Ok(Some(order)) => {
                info!("Order executed: {:?}", order);
                self.create_position(order, signal, position_side).await?;
            }
            Ok(None) => {
                warn!("Order execution returned no order");
            }
            Err(e) => {
                error!("Failed to execute order: {}", e);
                return Err(MonitorError::Other(format!("Order execution failed: {}", e)));
            }
        }
        
        Ok(())
    }
    
    async fn create_position(
        &self,
        order: Order,
        signal: TradingSignal,
        side: PositionSide,
    ) -> Result<()> {
        let stop_loss = self.risk_manager.get_stop_loss(signal.price, side.clone());
        let take_profit = self.risk_manager.get_take_profit(signal.price, side.clone());
        
        let position = Position {
            id: uuid::Uuid::new_v4(),
            symbol: signal.symbol.clone(),
            exchange: signal.exchange.clone(),
            side,
            quantity: order.quantity,
            entry_price: signal.price,
            current_price: signal.price,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            stop_loss: Some(stop_loss),
            take_profit: Some(take_profit),
            opened_at: chrono::Utc::now(),
            closed_at: None,
        };
        
        let position_key = format!("{}:{}", signal.exchange, signal.symbol);
        self.positions.insert(position_key, position);
        
        info!("Position created: {}/{} @ {}", signal.exchange, signal.symbol, signal.price);
        
        Ok(())
    }
    
    pub async fn update_positions(&self, symbol: &str, exchange: &str, price: f64) -> Result<()> {
        let position_key = format!("{}:{}", exchange, symbol);
        
        if let Some(mut position) = self.positions.get_mut(&position_key) {
            position.update_price(price);
            
            // Check stop loss
            if position.should_stop_loss() {
                info!("Stop loss triggered for {}/{}", exchange, symbol);
                self.close_position(&position_key).await?;
            }
            // Check take profit
            else if position.should_take_profit() {
                info!("Take profit triggered for {}/{}", exchange, symbol);
                self.close_position(&position_key).await?;
            }
        }
        
        Ok(())
    }
    
    async fn close_position(&self, position_key: &str) -> Result<()> {
        if let Some((_, position)) = self.positions.remove(position_key) {
            let side = match position.side {
                PositionSide::Long => OrderKind::Sell,
                PositionSide::Short => OrderKind::Buy,
            };
            
            let order_request = RequestOpen {
                instrument: position.symbol.clone(),
                exchange: position.exchange.clone(),
                kind: side,
                order_type: OrderType::Market,
                quantity: position.quantity,
                price: None,
                time_in_force: None,
                post_only: false,
                reduce_only: true,
            };
            
            match self.execution_client.open_order(order_request).await {
                Ok(Some(order)) => {
                    info!("Position closed: {:?}", order);
                    self.update_stats(position.unrealized_pnl);
                }
                Ok(None) => {
                    warn!("Close position returned no order");
                }
                Err(e) => {
                    error!("Failed to close position: {}", e);
                    // Re-insert position if close failed
                    self.positions.insert(position_key.to_string(), position);
                    return Err(MonitorError::Other(format!("Position close failed: {}", e)));
                }
            }
        }
        
        Ok(())
    }
    
    fn update_stats(&self, pnl: f64) {
        let mut stats = self.stats.write();
        stats.total_trades += 1;
        stats.total_pnl += pnl;
        
        if pnl > 0.0 {
            stats.winning_trades += 1;
        } else {
            stats.losing_trades += 1;
        }
        
        stats.win_rate = if stats.total_trades > 0 {
            stats.winning_trades as f64 / stats.total_trades as f64
        } else {
            0.0
        };
    }
    
    pub fn get_positions(&self) -> Vec<Position> {
        self.positions.iter().map(|p| p.clone()).collect()
    }
    
    pub fn get_stats(&self) -> TradingStats {
        self.stats.read().clone()
    }
    
    pub fn update_config(&self, config: TradingConfig) {
        *self.config.write() = config.clone();
        self.strategy.write().update_config(config);
    }
}