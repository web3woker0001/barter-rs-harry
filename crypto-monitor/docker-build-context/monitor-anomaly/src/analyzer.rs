use crate::{
    AnomalyDetection, AnomalyMetrics, AnomalySeverity, TimeSeriesData,
    metrics::{MetricsCalculator, TrendDirection},
};
use chrono::Utc;
use monitor_core::AnomalyType;
use std::collections::VecDeque;
use tracing::{debug, info};

pub struct MarketAnalyzer {
    symbol: String,
    exchange: String,
    metrics: MetricsCalculator,
    price_history: VecDeque<f64>,
    volume_history: VecDeque<f64>,
    max_history_size: usize,
}

impl MarketAnalyzer {
    pub fn new(symbol: String, exchange: String) -> Self {
        Self {
            symbol,
            exchange,
            metrics: MetricsCalculator::new(),
            price_history: VecDeque::new(),
            volume_history: VecDeque::new(),
            max_history_size: 1000,
        }
    }
    
    pub fn analyze_market_data(
        &mut self,
        price: f64,
        volume: f64,
    ) -> Vec<AnomalyDetection> {
        let mut anomalies = Vec::new();
        
        // Update history
        self.update_history(price, volume);
        
        // Update metrics
        let timestamp = Utc::now();
        self.metrics.add_data(
            "price",
            TimeSeriesData { timestamp, value: price },
            60,
        );
        self.metrics.add_data(
            "volume",
            TimeSeriesData { timestamp, value: volume },
            60,
        );
        
        // Check for various anomalies
        if let Some(anomaly) = self.check_unusual_volume() {
            anomalies.push(anomaly);
        }
        
        if let Some(anomaly) = self.check_price_manipulation() {
            anomalies.push(anomaly);
        }
        
        if let Some(anomaly) = self.check_flash_crash() {
            anomalies.push(anomaly);
        }
        
        if let Some(anomaly) = self.check_pump_dump() {
            anomalies.push(anomaly);
        }
        
        anomalies
    }
    
    fn update_history(&mut self, price: f64, volume: f64) {
        self.price_history.push_back(price);
        if self.price_history.len() > self.max_history_size {
            self.price_history.pop_front();
        }
        
        self.volume_history.push_back(volume);
        if self.volume_history.len() > self.max_history_size {
            self.volume_history.pop_front();
        }
    }
    
    fn check_unusual_volume(&self) -> Option<AnomalyDetection> {
        if self.volume_history.len() < 30 {
            return None;
        }
        
        let current_volume = *self.volume_history.back()?;
        let avg_volume: f64 = self.volume_history.iter().sum::<f64>() 
            / self.volume_history.len() as f64;
        
        let volume_ratio = current_volume / avg_volume;
        
        if volume_ratio > 5.0 {
            let description = format!(
                "Unusual volume detected for {}/{}: current volume is {:.1}x average",
                self.exchange, self.symbol, volume_ratio
            );
            
            info!("{}", description);
            
            return Some(AnomalyDetection {
                id: uuid::Uuid::new_v4(),
                timestamp: Utc::now(),
                symbol: self.symbol.clone(),
                exchange: self.exchange.clone(),
                anomaly_type: AnomalyType::UnusualActivity,
                severity: if volume_ratio > 10.0 {
                    AnomalySeverity::Critical
                } else {
                    AnomalySeverity::High
                },
                metrics: AnomalyMetrics {
                    current_value: current_volume,
                    expected_value: avg_volume,
                    deviation: current_volume - avg_volume,
                    z_score: None,
                    percentage_change: Some((volume_ratio - 1.0) * 100.0),
                    historical_avg: Some(avg_volume),
                    historical_std: None,
                },
                description,
            });
        }
        
        None
    }
    
    fn check_price_manipulation(&self) -> Option<AnomalyDetection> {
        if self.price_history.len() < 10 {
            return None;
        }
        
        // Check for rapid price changes with low volume
        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(10)
            .cloned()
            .collect();
        
        let price_change = (recent_prices[0] - recent_prices[9]) / recent_prices[9] * 100.0;
        let avg_volume: f64 = self.volume_history
            .iter()
            .rev()
            .take(10)
            .sum::<f64>() / 10.0;
        
        let historical_avg_volume: f64 = self.volume_history.iter().sum::<f64>() 
            / self.volume_history.len() as f64;
        
        // Suspicious if large price change with below-average volume
        if price_change.abs() > 3.0 && avg_volume < historical_avg_volume * 0.5 {
            let description = format!(
                "Potential price manipulation for {}/{}: {:.2}% price change with low volume",
                self.exchange, self.symbol, price_change
            );
            
            info!("{}", description);
            
            return Some(AnomalyDetection {
                id: uuid::Uuid::new_v4(),
                timestamp: Utc::now(),
                symbol: self.symbol.clone(),
                exchange: self.exchange.clone(),
                anomaly_type: AnomalyType::UnusualActivity,
                severity: AnomalySeverity::High,
                metrics: AnomalyMetrics {
                    current_value: recent_prices[0],
                    expected_value: recent_prices[9],
                    deviation: recent_prices[0] - recent_prices[9],
                    z_score: None,
                    percentage_change: Some(price_change),
                    historical_avg: Some(historical_avg_volume),
                    historical_std: None,
                },
                description,
            });
        }
        
        None
    }
    
    fn check_flash_crash(&self) -> Option<AnomalyDetection> {
        if self.price_history.len() < 5 {
            return None;
        }
        
        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect();
        
        let max_price = recent_prices.iter().cloned().fold(f64::MIN, f64::max);
        let min_price = recent_prices.iter().cloned().fold(f64::MAX, f64::min);
        let current_price = recent_prices[0];
        
        let drop_percentage = ((max_price - min_price) / max_price) * 100.0;
        
        if drop_percentage > 10.0 && current_price < max_price * 0.9 {
            let description = format!(
                "Flash crash detected for {}/{}: {:.2}% drop in 5 periods",
                self.exchange, self.symbol, drop_percentage
            );
            
            info!("{}", description);
            
            return Some(AnomalyDetection {
                id: uuid::Uuid::new_v4(),
                timestamp: Utc::now(),
                symbol: self.symbol.clone(),
                exchange: self.exchange.clone(),
                anomaly_type: AnomalyType::PriceSpike,
                severity: AnomalySeverity::Critical,
                metrics: AnomalyMetrics {
                    current_value: current_price,
                    expected_value: max_price,
                    deviation: current_price - max_price,
                    z_score: None,
                    percentage_change: Some(-drop_percentage),
                    historical_avg: None,
                    historical_std: None,
                },
                description,
            });
        }
        
        None
    }
    
    fn check_pump_dump(&self) -> Option<AnomalyDetection> {
        if self.price_history.len() < 20 || self.volume_history.len() < 20 {
            return None;
        }
        
        // Check for rapid price increase followed by decrease
        let prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(20)
            .cloned()
            .collect();
        
        let volumes: Vec<f64> = self.volume_history
            .iter()
            .rev()
            .take(20)
            .cloned()
            .collect();
        
        // Find peak
        let (peak_idx, peak_price) = prices
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, p)| (i, *p))
            .unwrap();
        
        // Check if it's a pump and dump pattern
        if peak_idx > 5 && peak_idx < 15 {
            let price_before = prices[19];
            let price_after = prices[0];
            let pump_percentage = ((peak_price - price_before) / price_before) * 100.0;
            let dump_percentage = ((peak_price - price_after) / peak_price) * 100.0;
            
            if pump_percentage > 20.0 && dump_percentage > 15.0 {
                let description = format!(
                    "Pump and dump pattern detected for {}/{}: +{:.1}% pump, -{:.1}% dump",
                    self.exchange, self.symbol, pump_percentage, dump_percentage
                );
                
                info!("{}", description);
                
                return Some(AnomalyDetection {
                    id: uuid::Uuid::new_v4(),
                    timestamp: Utc::now(),
                    symbol: self.symbol.clone(),
                    exchange: self.exchange.clone(),
                    anomaly_type: AnomalyType::UnusualActivity,
                    severity: AnomalySeverity::Critical,
                    metrics: AnomalyMetrics {
                        current_value: price_after,
                        expected_value: price_before,
                        deviation: price_after - price_before,
                        z_score: None,
                        percentage_change: Some(pump_percentage),
                        historical_avg: None,
                        historical_std: None,
                    },
                    description,
                });
            }
        }
        
        None
    }
}