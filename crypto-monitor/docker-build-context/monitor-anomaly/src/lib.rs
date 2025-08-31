pub mod detector;
pub mod metrics;
pub mod analyzer;

use chrono::{DateTime, Utc};
use monitor_core::{AnomalyType, MonitorError, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetection {
    pub id: uuid::Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub exchange: String,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub metrics: AnomalyMetrics,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyMetrics {
    pub current_value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub z_score: Option<f64>,
    pub percentage_change: Option<f64>,
    pub historical_avg: Option<f64>,
    pub historical_std: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TimeSeriesData {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct TimeSeriesWindow {
    pub data: VecDeque<TimeSeriesData>,
    pub max_size: usize,
    pub sum: f64,
    pub sum_squared: f64,
}

impl TimeSeriesWindow {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_size),
            max_size,
            sum: 0.0,
            sum_squared: 0.0,
        }
    }
    
    pub fn push(&mut self, point: TimeSeriesData) {
        if self.data.len() >= self.max_size {
            if let Some(old) = self.data.pop_front() {
                self.sum -= old.value;
                self.sum_squared -= old.value * old.value;
            }
        }
        
        self.sum += point.value;
        self.sum_squared += point.value * point.value;
        self.data.push_back(point);
    }
    
    pub fn mean(&self) -> f64 {
        if self.data.is_empty() {
            0.0
        } else {
            self.sum / self.data.len() as f64
        }
    }
    
    pub fn std_dev(&self) -> f64 {
        if self.data.len() < 2 {
            0.0
        } else {
            let mean = self.mean();
            let variance = (self.sum_squared / self.data.len() as f64) - (mean * mean);
            variance.max(0.0).sqrt()
        }
    }
    
    pub fn z_score(&self, value: f64) -> f64 {
        let std_dev = self.std_dev();
        if std_dev == 0.0 {
            0.0
        } else {
            (value - self.mean()) / std_dev
        }
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

pub trait AnomalyDetector: Send + Sync {
    fn detect(&mut self, data: &TimeSeriesData) -> Option<AnomalyDetection>;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAnomalyConfig {
    pub z_score_threshold: f64,
    pub min_percentage_change: f64,
    pub window_size: usize,
    pub min_samples: usize,
}

impl Default for VolumeAnomalyConfig {
    fn default() -> Self {
        Self {
            z_score_threshold: 3.0,
            min_percentage_change: 200.0,
            window_size: 60,
            min_samples: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAnomalyConfig {
    pub percentage_threshold: f64,
    pub z_score_threshold: f64,
    pub window_size: usize,
    pub min_samples: usize,
}

impl Default for PriceAnomalyConfig {
    fn default() -> Self {
        Self {
            percentage_threshold: 5.0,
            z_score_threshold: 3.0,
            window_size: 60,
            min_samples: 30,
        }
    }
}