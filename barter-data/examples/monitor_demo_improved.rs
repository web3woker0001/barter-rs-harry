/// 改进版实时加密货币监控系统
/// 
/// 改进内容：
/// 1. 优化异常检测敏感度
/// 2. 动态调整阈值
/// 3. 过滤 WebSocket ping/pong 错误
/// 4. 更好的统计展示

use barter_data::{
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
        okx::Okx,
    },
    streams::{Streams, reconnect::stream::ReconnectingStream},
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

/// 改进的监控配置
#[derive(Debug, Clone)]
struct MonitorConfig {
    /// 价格变化阈值（百分比）
    price_change_threshold: f64,
    /// 成交量异常初始倍数
    volume_anomaly_multiplier_base: f64,
    /// 成交量异常最大倍数
    volume_anomaly_multiplier_max: f64,
    /// 历史数据窗口大小
    window_size: usize,
    /// 统计报告间隔（秒）
    report_interval_secs: u64,
    /// 最小样本数（避免初期误报）
    min_samples: usize,
    /// 动态阈值调整
    dynamic_threshold: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            price_change_threshold: 3.0,         // 3% 价格变化触发告警（提高阈值）
            volume_anomaly_multiplier_base: 5.0, // 基础倍数提高到5倍
            volume_anomaly_multiplier_max: 20.0, // 最大20倍
            window_size: 200,                    // 增加窗口大小
            report_interval_secs: 10,
            min_samples: 20,                     // 至少20个样本才开始检测
            dynamic_threshold: true,             // 启用动态阈值
        }
    }
}

/// 市场数据点
#[derive(Debug, Clone)]
struct MarketDataPoint {
    timestamp: DateTime<Utc>,
    price: f64,
    volume: f64,
    exchange: String,
    symbol: String,
    market_type: String,
}

/// 统计指标
#[derive(Debug, Clone)]
struct Statistics {
    mean: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    percentile_95: f64,
}

impl Statistics {
    fn calculate(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let percentile_95 = sorted[(sorted.len() as f64 * 0.95) as usize];
        
        Some(Self {
            mean,
            std_dev,
            min,
            max,
            percentile_95,
        })
    }
}

/// 改进的交易对监控器
#[derive(Debug)]
struct SymbolMonitor {
    symbol: String,
    data_points: VecDeque<MarketDataPoint>,
    last_price: f64,
    total_volume: f64,
    trade_count: u64,
    price_changes: VecDeque<f64>,
    volume_history: VecDeque<f64>,
    anomalies_detected: u64,
    false_positives: u64,
    last_alert_time: Option<Instant>,
    dynamic_volume_threshold: f64,
}

impl SymbolMonitor {
    fn new(symbol: String, config: &MonitorConfig) -> Self {
        Self {
            symbol,
            data_points: VecDeque::with_capacity(config.window_size),
            last_price: 0.0,
            total_volume: 0.0,
            trade_count: 0,
            price_changes: VecDeque::with_capacity(config.window_size),
            volume_history: VecDeque::with_capacity(config.window_size),
            anomalies_detected: 0,
            false_positives: 0,
            last_alert_time: None,
            dynamic_volume_threshold: config.volume_anomaly_multiplier_base,
        }
    }
    
    fn add_data_point(&mut self, point: MarketDataPoint, config: &MonitorConfig) -> Option<String> {
        let mut alert = None;
        
        // 更新交易数据
        self.trade_count += 1;
        self.total_volume += point.volume;
        
        // 维护历史数据窗口
        if self.data_points.len() >= config.window_size {
            self.data_points.pop_front();
        }
        self.data_points.push_back(point.clone());
        
        if self.volume_history.len() >= config.window_size {
            self.volume_history.pop_front();
        }
        self.volume_history.push_back(point.volume);
        
        // 需要足够的样本才开始检测
        if self.data_points.len() < config.min_samples {
            self.last_price = point.price;
            return None;
        }
        
        // 避免告警过于频繁（至少间隔2秒）
        if let Some(last_time) = self.last_alert_time {
            if last_time.elapsed() < Duration::from_secs(2) {
                self.last_price = point.price;
                return None;
            }
        }
        
        // 价格异常检测（使用改进的算法）
        if self.last_price > 0.0 {
            let price_change_pct = ((point.price - self.last_price) / self.last_price * 100.0).abs();
            
            // 记录价格变化
            if self.price_changes.len() >= config.window_size {
                self.price_changes.pop_front();
            }
            self.price_changes.push_back(price_change_pct);
            
            // 计算价格变化统计
            if let Some(price_stats) = Statistics::calculate(&self.price_changes.iter().copied().collect::<Vec<_>>()) {
                // 使用动态阈值：均值 + 2倍标准差
                let dynamic_price_threshold = if config.dynamic_threshold {
                    (price_stats.mean + 2.0 * price_stats.std_dev).max(config.price_change_threshold)
                } else {
                    config.price_change_threshold
                };
                
                if price_change_pct > dynamic_price_threshold && price_change_pct > price_stats.percentile_95 {
                    alert = Some(format!(
                        "⚠️ 价格异常！{} {} 变化 {:.2}% (${:.2} -> ${:.2}) [阈值: {:.2}%]",
                        point.exchange, self.symbol, price_change_pct, 
                        self.last_price, point.price, dynamic_price_threshold
                    ));
                    self.anomalies_detected += 1;
                    self.last_alert_time = Some(Instant::now());
                }
            }
        }
        
        // 成交量异常检测（改进算法）
        if let Some(volume_stats) = Statistics::calculate(&self.volume_history.iter().copied().collect::<Vec<_>>()) {
            // 动态调整成交量阈值
            if config.dynamic_threshold {
                // 使用 IQR（四分位距）方法
                let iqr_multiplier = 1.5 + (volume_stats.std_dev / volume_stats.mean).min(2.0);
                self.dynamic_volume_threshold = (config.volume_anomaly_multiplier_base * iqr_multiplier)
                    .min(config.volume_anomaly_multiplier_max);
            }
            
            // 使用百分位数和动态阈值
            let volume_threshold = volume_stats.mean * self.dynamic_volume_threshold;
            
            if point.volume > volume_threshold && point.volume > volume_stats.percentile_95 * 1.5 {
                let volume_alert = format!(
                    "📊 成交量异常！{} {} 成交量 {:.4} (均值: {:.4}, {:.1}倍)",
                    point.exchange, self.symbol, point.volume, 
                    volume_stats.mean, point.volume / volume_stats.mean
                );
                
                if alert.is_none() {
                    alert = Some(volume_alert);
                } else {
                    alert = Some(format!("{}\n{}", alert.unwrap(), volume_alert));
                }
                self.anomalies_detected += 1;
                self.last_alert_time = Some(Instant::now());
            }
        }
        
        self.last_price = point.price;
        alert
    }
    
    fn get_statistics(&self) -> String {
        let price_volatility = if !self.price_changes.is_empty() {
            let price_vec: Vec<f64> = self.price_changes.iter().copied().collect();
            Statistics::calculate(&price_vec)
                .map(|s| s.std_dev)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        
        let avg_volume = if !self.volume_history.is_empty() {
            self.volume_history.iter().sum::<f64>() / self.volume_history.len() as f64
        } else {
            0.0
        };
        
        let detection_rate = if self.trade_count > 0 {
            (self.anomalies_detected as f64 / self.trade_count as f64 * 100.0)
        } else {
            0.0
        };
        
        format!(
            "📈 {} - 价格: ${:.2} | 均量: {:.4} | 总量: {:.2} | 交易: {} | 波动: {:.3}% | 异常: {} ({:.2}%)",
            self.symbol, self.last_price, avg_volume, self.total_volume, 
            self.trade_count, price_volatility, self.anomalies_detected, detection_rate
        )
    }
}

/// 改进的监控系统
struct MonitoringSystem {
    config: MonitorConfig,
    monitors: Arc<Mutex<HashMap<String, SymbolMonitor>>>,
    start_time: Instant,
    total_events: Arc<Mutex<u64>>,
    error_count: Arc<Mutex<u64>>,
    filtered_errors: Arc<Mutex<u64>>,
}

impl MonitoringSystem {
    fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            monitors: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
            total_events: Arc::new(Mutex::new(0)),
            error_count: Arc::new(Mutex::new(0)),
            filtered_errors: Arc::new(Mutex::new(0)),
        }
    }
    
    async fn process_trade(&self, exchange: String, symbol: String, market_type: String, price: f64, volume: f64) {
        let point = MarketDataPoint {
            timestamp: Utc::now(),
            price,
            volume,
            exchange: exchange.clone(),
            symbol: symbol.clone(),
            market_type,
        };
        
        let mut monitors = self.monitors.lock().await;
        let monitor = monitors.entry(symbol.clone())
            .or_insert_with(|| SymbolMonitor::new(symbol, &self.config));
        
        if let Some(alert) = monitor.add_data_point(point, &self.config) {
            warn!("{}", alert);
        }
        
        let mut total = self.total_events.lock().await;
        *total += 1;
    }
    
    async fn handle_error(&self, error_msg: &str) {
        // 过滤已知的 ping/pong 错误
        if error_msg.contains("pong") || error_msg.contains("ping") || error_msg.contains("subscription_id") {
            let mut filtered = self.filtered_errors.lock().await;
            *filtered += 1;
            debug!("Filtered known error: {}", error_msg);
        } else {
            let mut errors = self.error_count.lock().await;
            *errors += 1;
            error!("Stream error: {}", error_msg);
        }
    }
    
    async fn generate_report(&self) {
        let monitors = self.monitors.lock().await;
        let total_events = *self.total_events.lock().await;
        let error_count = *self.error_count.lock().await;
        let filtered_errors = *self.filtered_errors.lock().await;
        let elapsed = self.start_time.elapsed().as_secs();
        
        println!("\n================== 监控系统报告 ==================");
        println!("运行时间: {} 秒 | 总事件: {} | 速率: {:.1} 事件/秒", 
                 elapsed, total_events, total_events as f64 / elapsed.max(1) as f64);
        println!("错误统计: {} 个错误 | {} 个已过滤", error_count, filtered_errors);
        println!("--------------------------------------------------");
        
        for (_, monitor) in monitors.iter() {
            println!("{}", monitor.get_statistics());
        }
        
        // 显示监控配置
        println!("--------------------------------------------------");
        println!("监控配置: 价格阈值 {:.1}% | 成交量倍数 {:.1}-{:.1}x | 动态阈值: {}",
                 self.config.price_change_threshold,
                 self.config.volume_anomaly_multiplier_base,
                 self.config.volume_anomaly_multiplier_max,
                 if self.config.dynamic_threshold { "启用" } else { "禁用" });
        println!("==================================================\n");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志（降低日志级别）
    init_logging();
    
    println!("\n🚀 启动改进版加密货币实时监控系统 v2.0");
    println!("=========================================");
    println!("监控交易所: Binance, OKX, Bybit");
    println!("监控币种: BTC/USDT, ETH/USDT");
    println!("改进功能:");
    println!("  ✅ 优化异常检测敏感度");
    println!("  ✅ 动态阈值调整");
    println!("  ✅ 过滤 WebSocket 噪音");
    println!("  ✅ 改进统计显示");
    println!("=========================================\n");
    
    // 创建监控系统
    let config = MonitorConfig::default();
    let monitoring_system = Arc::new(MonitoringSystem::new(config.clone()));
    
    // 构建数据流
    info!("初始化交易所数据流...");
    let streams = Streams::<PublicTrades>::builder()
        // Binance
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])
        // OKX
        .subscribe([
            (Okx, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (Okx, "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])
        // Bybit
        .subscribe([
            (BybitSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (BybitSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])
        .init()
        .await?;
    
    info!("✅ 数据流初始化成功，开始监控...\n");
    
    // 合并流（改进错误处理）
    let error_handler = Arc::clone(&monitoring_system);
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(move |error| {
            let error_handler = Arc::clone(&error_handler);
            let error_msg = format!("{:?}", error);
            tokio::spawn(async move {
                error_handler.handle_error(&error_msg).await;
            });
        });
    
    // 启动定期报告任务
    let report_system = Arc::clone(&monitoring_system);
    let report_interval = config.report_interval_secs;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(report_interval));
        loop {
            interval.tick().await;
            report_system.generate_report().await;
        }
    });
    
    // 主监控循环
    let test_duration = Duration::from_secs(60); // 运行60秒
    let timeout = tokio::time::sleep(test_duration);
    tokio::pin!(timeout);
    
    info!("监控系统运行中... (运行时间: {} 秒)", test_duration.as_secs());
    
    loop {
        tokio::select! {
            _ = &mut timeout => {
                info!("\n⏰ 监控时间结束");
                break;
            }
            event = joined_stream.next() => {
                if let Some(event) = event {
                    match event {
                        barter_data::streams::reconnect::Event::Item(market_event) => {
                            // 提取交易所信息
                            let debug_str = format!("{:?}", market_event);
                            let exchange = if debug_str.contains("Binance") {
                                "Binance"
                            } else if debug_str.contains("Okx") {
                                "OKX"
                            } else if debug_str.contains("Bybit") {
                                "Bybit"
                            } else {
                                "Unknown"
                            };
                            
                            let symbol = format!("{}/{}",
                                market_event.instrument.base,
                                market_event.instrument.quote
                            ).to_uppercase();
                            
                            let market_type = match market_event.instrument.kind {
                                MarketDataInstrumentKind::Spot => "Spot",
                                MarketDataInstrumentKind::Perpetual => "Futures",
                                _ => "Unknown",
                            };
                            
                            // 处理交易数据
                            monitoring_system.process_trade(
                                exchange.to_string(),
                                symbol,
                                market_type.to_string(),
                                market_event.kind.price,
                                market_event.kind.amount,
                            ).await;
                        },
                        barter_data::streams::reconnect::Event::Reconnecting(exchange_id) => {
                            warn!("交易所重连中: {:?}", exchange_id);
                        }
                    }
                }
            }
        }
    }
    
    // 生成最终报告
    println!("\n🏁 监控系统关闭，生成最终报告...");
    monitoring_system.generate_report().await;
    
    println!("\n✨ 监控系统已优雅关闭");
    
    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .with_env_var("RUST_LOG")
                .from_env_lossy(),
        )
        .with_ansi(true)
        .compact()
        .init()
}