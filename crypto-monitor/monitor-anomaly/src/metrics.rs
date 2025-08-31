use crate::{TimeSeriesData, TimeSeriesWindow};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::debug;

pub struct MetricsCalculator {
    windows: HashMap<String, TimeSeriesWindow>,
}

impl MetricsCalculator {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }
    
    pub fn add_data(&mut self, key: &str, data: TimeSeriesData, window_size: usize) {
        let window = self.windows.entry(key.to_string())
            .or_insert_with(|| TimeSeriesWindow::new(window_size));
        window.push(data);
    }
    
    pub fn calculate_sma(&self, key: &str) -> Option<f64> {
        self.windows.get(key).map(|w| w.mean())
    }
    
    pub fn calculate_ema(&self, key: &str, alpha: f64) -> Option<f64> {
        self.windows.get(key).and_then(|w| {
            if w.is_empty() {
                return None;
            }
            
            let mut ema = w.data[0].value;
            for i in 1..w.data.len() {
                ema = alpha * w.data[i].value + (1.0 - alpha) * ema;
            }
            Some(ema)
        })
    }
    
    pub fn calculate_bollinger_bands(&self, key: &str) -> Option<(f64, f64, f64)> {
        self.windows.get(key).map(|w| {
            let mean = w.mean();
            let std_dev = w.std_dev();
            let upper = mean + 2.0 * std_dev;
            let lower = mean - 2.0 * std_dev;
            (lower, mean, upper)
        })
    }
    
    pub fn calculate_rsi(&self, key: &str, period: usize) -> Option<f64> {
        self.windows.get(key).and_then(|w| {
            if w.data.len() < period + 1 {
                return None;
            }
            
            let mut gains = 0.0;
            let mut losses = 0.0;
            
            for i in 1..=period {
                let change = w.data[i].value - w.data[i-1].value;
                if change > 0.0 {
                    gains += change;
                } else {
                    losses -= change;
                }
            }
            
            let avg_gain = gains / period as f64;
            let avg_loss = losses / period as f64;
            
            if avg_loss == 0.0 {
                return Some(100.0);
            }
            
            let rs = avg_gain / avg_loss;
            Some(100.0 - (100.0 / (1.0 + rs)))
        })
    }
    
    pub fn calculate_macd(&self, key: &str) -> Option<(f64, f64, f64)> {
        let ema12 = self.calculate_ema(key, 2.0 / 13.0)?;
        let ema26 = self.calculate_ema(key, 2.0 / 27.0)?;
        let macd = ema12 - ema26;
        let signal = macd * 0.2; // Simplified signal line
        let histogram = macd - signal;
        Some((macd, signal, histogram))
    }
    
    pub fn calculate_volatility(&self, key: &str) -> Option<f64> {
        self.windows.get(key).map(|w| w.std_dev())
    }
    
    pub fn detect_trend(&self, key: &str) -> Option<TrendDirection> {
        self.windows.get(key).and_then(|w| {
            if w.data.len() < 3 {
                return None;
            }
            
            let recent = &w.data[w.data.len() - 3..];
            let first = recent[0].value;
            let last = recent[recent.len() - 1].value;
            
            if last > first * 1.01 {
                Some(TrendDirection::Up)
            } else if last < first * 0.99 {
                Some(TrendDirection::Down)
            } else {
                Some(TrendDirection::Sideways)
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    Up,
    Down,
    Sideways,
}

pub struct VolumeProfile {
    pub levels: Vec<PriceLevel>,
    pub poc: f64, // Point of Control
    pub vah: f64, // Value Area High
    pub val: f64, // Value Area Low
}

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub volume: f64,
}

impl VolumeProfile {
    pub fn calculate(trades: &[(f64, f64)]) -> Option<Self> {
        if trades.is_empty() {
            return None;
        }
        
        // Simplified volume profile calculation
        let mut price_volumes: HashMap<i64, f64> = HashMap::new();
        
        for (price, volume) in trades {
            let price_level = (*price * 100.0) as i64;
            *price_volumes.entry(price_level).or_insert(0.0) += volume;
        }
        
        let mut levels: Vec<PriceLevel> = price_volumes
            .into_iter()
            .map(|(p, v)| PriceLevel {
                price: p as f64 / 100.0,
                volume: v,
            })
            .collect();
        
        levels.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap());
        
        if levels.is_empty() {
            return None;
        }
        
        let poc = levels[0].price;
        let total_volume: f64 = levels.iter().map(|l| l.volume).sum();
        let value_area_volume = total_volume * 0.7;
        
        let mut accumulated_volume = 0.0;
        let mut vah = poc;
        let mut val = poc;
        
        for level in &levels {
            accumulated_volume += level.volume;
            if accumulated_volume <= value_area_volume {
                vah = vah.max(level.price);
                val = val.min(level.price);
            }
        }
        
        Some(VolumeProfile {
            levels,
            poc,
            vah,
            val,
        })
    }
}