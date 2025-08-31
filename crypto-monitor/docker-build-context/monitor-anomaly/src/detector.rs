use crate::{
    AnomalyDetection, AnomalyDetector, AnomalyMetrics, AnomalySeverity,
    PriceAnomalyConfig, TimeSeriesData, TimeSeriesWindow, VolumeAnomalyConfig,
};
use chrono::Utc;
use monitor_core::AnomalyType;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct VolumeAnomalyDetector {
    config: VolumeAnomalyConfig,
    symbol: String,
    exchange: String,
    window: TimeSeriesWindow,
}

impl VolumeAnomalyDetector {
    pub fn new(config: VolumeAnomalyConfig, symbol: String, exchange: String) -> Self {
        Self {
            window: TimeSeriesWindow::new(config.window_size),
            config,
            symbol,
            exchange,
        }
    }
}

impl AnomalyDetector for VolumeAnomalyDetector {
    fn detect(&mut self, data: &TimeSeriesData) -> Option<AnomalyDetection> {
        self.window.push(data.clone());
        
        if self.window.len() < self.config.min_samples {
            return None;
        }
        
        let mean = self.window.mean();
        let std_dev = self.window.std_dev();
        let z_score = self.window.z_score(data.value);
        
        let percentage_change = if mean > 0.0 {
            ((data.value - mean) / mean) * 100.0
        } else {
            0.0
        };
        
        if z_score.abs() >= self.config.z_score_threshold
            && percentage_change.abs() >= self.config.min_percentage_change
        {
            let severity = match z_score.abs() {
                z if z >= 5.0 => AnomalySeverity::Critical,
                z if z >= 4.0 => AnomalySeverity::High,
                z if z >= 3.0 => AnomalySeverity::Medium,
                _ => AnomalySeverity::Low,
            };
            
            let description = format!(
                "Volume anomaly detected for {}/{}: current volume {:.2} is {:.1}% {} average ({:.2}), Z-score: {:.2}",
                self.exchange,
                self.symbol,
                data.value,
                percentage_change.abs(),
                if percentage_change > 0.0 { "above" } else { "below" },
                mean,
                z_score
            );
            
            info!("{}", description);
            
            Some(AnomalyDetection {
                id: uuid::Uuid::new_v4(),
                timestamp: data.timestamp,
                symbol: self.symbol.clone(),
                exchange: self.exchange.clone(),
                anomaly_type: AnomalyType::VolumeSpike,
                severity,
                metrics: AnomalyMetrics {
                    current_value: data.value,
                    expected_value: mean,
                    deviation: data.value - mean,
                    z_score: Some(z_score),
                    percentage_change: Some(percentage_change),
                    historical_avg: Some(mean),
                    historical_std: Some(std_dev),
                },
                description,
            })
        } else {
            None
        }
    }
    
    fn reset(&mut self) {
        self.window = TimeSeriesWindow::new(self.config.window_size);
    }
}

pub struct PriceAnomalyDetector {
    config: PriceAnomalyConfig,
    symbol: String,
    exchange: String,
    window: TimeSeriesWindow,
    last_price: Option<f64>,
}

impl PriceAnomalyDetector {
    pub fn new(config: PriceAnomalyConfig, symbol: String, exchange: String) -> Self {
        Self {
            window: TimeSeriesWindow::new(config.window_size),
            config,
            symbol,
            exchange,
            last_price: None,
        }
    }
}

impl AnomalyDetector for PriceAnomalyDetector {
    fn detect(&mut self, data: &TimeSeriesData) -> Option<AnomalyDetection> {
        let current_price = data.value;
        
        self.window.push(data.clone());
        
        if self.window.len() < self.config.min_samples {
            self.last_price = Some(current_price);
            return None;
        }
        
        let mean = self.window.mean();
        let std_dev = self.window.std_dev();
        let z_score = self.window.z_score(current_price);
        
        let percentage_change = if let Some(last) = self.last_price {
            if last > 0.0 {
                ((current_price - last) / last) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        self.last_price = Some(current_price);
        
        if percentage_change.abs() >= self.config.percentage_threshold
            || z_score.abs() >= self.config.z_score_threshold
        {
            let severity = match (percentage_change.abs(), z_score.abs()) {
                (p, z) if p >= 10.0 || z >= 5.0 => AnomalySeverity::Critical,
                (p, z) if p >= 7.0 || z >= 4.0 => AnomalySeverity::High,
                (p, z) if p >= 5.0 || z >= 3.0 => AnomalySeverity::Medium,
                _ => AnomalySeverity::Low,
            };
            
            let description = format!(
                "Price anomaly detected for {}/{}: price moved {:.2}% from {:.4} to {:.4}, Z-score: {:.2}",
                self.exchange,
                self.symbol,
                percentage_change,
                self.last_price.unwrap_or(0.0),
                current_price,
                z_score
            );
            
            info!("{}", description);
            
            Some(AnomalyDetection {
                id: uuid::Uuid::new_v4(),
                timestamp: data.timestamp,
                symbol: self.symbol.clone(),
                exchange: self.exchange.clone(),
                anomaly_type: AnomalyType::PriceSpike,
                severity,
                metrics: AnomalyMetrics {
                    current_value: current_price,
                    expected_value: mean,
                    deviation: current_price - mean,
                    z_score: Some(z_score),
                    percentage_change: Some(percentage_change),
                    historical_avg: Some(mean),
                    historical_std: Some(std_dev),
                },
                description,
            })
        } else {
            None
        }
    }
    
    fn reset(&mut self) {
        self.window = TimeSeriesWindow::new(self.config.window_size);
        self.last_price = None;
    }
}

pub struct CompositeAnomalyDetector {
    detectors: Vec<Box<dyn AnomalyDetector>>,
}

impl CompositeAnomalyDetector {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }
    
    pub fn add_detector(&mut self, detector: Box<dyn AnomalyDetector>) {
        self.detectors.push(detector);
    }
    
    pub fn detect_all(&mut self, data: &TimeSeriesData) -> Vec<AnomalyDetection> {
        self.detectors
            .iter_mut()
            .filter_map(|d| d.detect(data))
            .collect()
    }
    
    pub fn reset_all(&mut self) {
        for detector in &mut self.detectors {
            detector.reset();
        }
    }
}

pub struct AnomalyDetectorManager {
    detectors: Arc<RwLock<HashMap<String, CompositeAnomalyDetector>>>,
    volume_config: VolumeAnomalyConfig,
    price_config: PriceAnomalyConfig,
}

impl AnomalyDetectorManager {
    pub fn new(
        volume_config: VolumeAnomalyConfig,
        price_config: PriceAnomalyConfig,
    ) -> Self {
        Self {
            detectors: Arc::new(RwLock::new(HashMap::new())),
            volume_config,
            price_config,
        }
    }
    
    pub fn get_or_create_detector(
        &self,
        symbol: &str,
        exchange: &str,
    ) -> CompositeAnomalyDetector {
        let key = format!("{}:{}", exchange, symbol);
        
        let mut detectors = self.detectors.write();
        
        detectors.entry(key).or_insert_with(|| {
            let mut composite = CompositeAnomalyDetector::new();
            
            composite.add_detector(Box::new(VolumeAnomalyDetector::new(
                self.volume_config.clone(),
                symbol.to_string(),
                exchange.to_string(),
            )));
            
            composite.add_detector(Box::new(PriceAnomalyDetector::new(
                self.price_config.clone(),
                symbol.to_string(),
                exchange.to_string(),
            )));
            
            composite
        }).clone()
    }
    
    pub fn process_data(
        &self,
        symbol: &str,
        exchange: &str,
        data: &TimeSeriesData,
    ) -> Vec<AnomalyDetection> {
        let key = format!("{}:{}", exchange, symbol);
        
        let mut detectors = self.detectors.write();
        
        let composite = detectors.entry(key).or_insert_with(|| {
            let mut composite = CompositeAnomalyDetector::new();
            
            composite.add_detector(Box::new(VolumeAnomalyDetector::new(
                self.volume_config.clone(),
                symbol.to_string(),
                exchange.to_string(),
            )));
            
            composite.add_detector(Box::new(PriceAnomalyDetector::new(
                self.price_config.clone(),
                symbol.to_string(),
                exchange.to_string(),
            )));
            
            composite
        });
        
        composite.detect_all(data)
    }
    
    pub fn reset(&self, symbol: &str, exchange: &str) {
        let key = format!("{}:{}", exchange, symbol);
        
        if let Some(detector) = self.detectors.write().get_mut(&key) {
            detector.reset_all();
        }
    }
    
    pub fn reset_all(&self) {
        let mut detectors = self.detectors.write();
        for detector in detectors.values_mut() {
            detector.reset_all();
        }
    }
}