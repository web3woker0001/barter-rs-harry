use crate::{SignalStrength, SignalType, TradingSignal, TradingStrategy};
use monitor_anomaly::AnomalyDetection;
use monitor_core::TradingConfig;

pub struct AnomalyBasedStrategy {
    config: TradingConfig,
}

impl AnomalyBasedStrategy {
    pub fn new(config: TradingConfig) -> Self {
        Self { config }
    }
}

impl TradingStrategy for AnomalyBasedStrategy {
    fn analyze(&mut self, anomaly: &AnomalyDetection) -> Option<TradingSignal> {
        if !self.config.auto_trading_enabled {
            return None;
        }
        
        // Simple strategy based on anomaly type and severity
        let (signal_type, strength) = match &anomaly.anomaly_type {
            monitor_core::AnomalyType::VolumeSpike => {
                // High volume might indicate trend start
                match anomaly.severity {
                    monitor_anomaly::AnomalySeverity::Critical => {
                        (SignalType::Buy, SignalStrength::Strong)
                    }
                    monitor_anomaly::AnomalySeverity::High => {
                        (SignalType::Buy, SignalStrength::Medium)
                    }
                    _ => return None,
                }
            }
            monitor_core::AnomalyType::PriceSpike => {
                // Price spike might be overreaction
                if let Some(pct) = anomaly.metrics.percentage_change {
                    if pct < -5.0 {
                        (SignalType::Buy, SignalStrength::Medium)
                    } else if pct > 10.0 {
                        (SignalType::Sell, SignalStrength::Medium)
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        
        Some(TradingSignal {
            id: uuid::Uuid::new_v4(),
            timestamp: anomaly.timestamp,
            symbol: anomaly.symbol.clone(),
            exchange: anomaly.exchange.clone(),
            signal_type,
            strength,
            price: anomaly.metrics.current_value,
            reason: format!("Anomaly detected: {}", anomaly.description),
            anomaly_id: Some(anomaly.id),
        })
    }
    
    fn update_config(&mut self, config: TradingConfig) {
        self.config = config;
    }
}