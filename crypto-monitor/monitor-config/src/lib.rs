use config::{Config, ConfigError, Environment, File};
use monitor_core::{MonitorConfig, MonitorError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

pub struct ConfigManager {
    config: Config,
    monitor_config: MonitorConfig,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        Self::from_file("config.yaml")
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = Config::builder()
            .add_source(File::from(path.as_ref()))
            .add_source(Environment::with_prefix("CRYPTO_MONITOR"))
            .build()
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        let monitor_config: MonitorConfig = config
            .try_deserialize()
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        info!("Configuration loaded successfully");
        
        Ok(Self {
            config,
            monitor_config,
        })
    }
    
    pub fn from_env() -> Result<Self> {
        let config = Config::builder()
            .add_source(Environment::with_prefix("CRYPTO_MONITOR"))
            .build()
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        let monitor_config: MonitorConfig = config
            .try_deserialize()
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        info!("Configuration loaded from environment");
        
        Ok(Self {
            config,
            monitor_config,
        })
    }
    
    pub fn get_config(&self) -> &MonitorConfig {
        &self.monitor_config
    }
    
    pub fn get_config_mut(&mut self) -> &mut MonitorConfig {
        &mut self.monitor_config
    }
    
    pub fn reload(&mut self) -> Result<()> {
        self.monitor_config = self.config
            .try_deserialize()
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        info!("Configuration reloaded");
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate exchanges
        if self.monitor_config.exchanges.is_empty() {
            return Err(MonitorError::Configuration(
                "No exchanges configured".to_string()
            ));
        }
        
        // Validate database
        if self.monitor_config.database.url.is_empty() {
            return Err(MonitorError::Configuration(
                "Database URL not configured".to_string()
            ));
        }
        
        // Validate Fluvio
        if self.monitor_config.fluvio.endpoint.is_empty() {
            return Err(MonitorError::Configuration(
                "Fluvio endpoint not configured".to_string()
            ));
        }
        
        info!("Configuration validation passed");
        Ok(())
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = serde_yaml::to_string(&self.monitor_config)
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        std::fs::write(path, yaml)
            .map_err(|e| MonitorError::Configuration(e.to_string()))?;
        
        info!("Configuration saved to file");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub debug_mode: bool,
    pub dry_run: bool,
    pub backtest_mode: bool,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            debug_mode: false,
            dry_run: false,
            backtest_mode: false,
            start_time: None,
            end_time: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_runtime_config() {
        let config = RuntimeConfig::default();
        assert!(!config.debug_mode);
        assert!(!config.dry_run);
        assert!(!config.backtest_mode);
    }
}