/// 实时加密货币监控系统演示
/// 
/// 功能：
/// 1. 实时监控多个交易所的价格和成交量
/// 2. 异常检测（价格突变、成交量异常）
/// 3. 实时统计和报告
/// 4. 自动告警功能

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
use tracing::{error, info, warn};

/// 监控配置
#[derive(Debug, Clone)]
struct MonitorConfig {
    /// 价格变化阈值（百分比）
    price_change_threshold: f64,
    /// 成交量异常倍数（相对于平均值）
    volume_anomaly_multiplier: f64,
    /// 历史数据窗口大小
    window_size: usize,
    /// 统计报告间隔（秒）
    report_interval_secs: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            price_change_threshold: 2.0,      // 2% 价格变化触发告警
            volume_anomaly_multiplier: 3.0,   // 3倍平均成交量触发告警
            window_size: 100,                 // 保留最近100条记录
            report_interval_secs: 10,         // 每10秒生成报告
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

/// 交易对监控器
#[derive(Debug)]
struct SymbolMonitor {
    symbol: String,
    data_points: VecDeque<MarketDataPoint>,
    last_price: f64,
    total_volume: f64,
    trade_count: u64,
    price_changes: Vec<f64>,
    anomalies_detected: u64,
}

impl SymbolMonitor {
    fn new(symbol: String) -> Self {
        Self {
            symbol,
            data_points: VecDeque::new(),
            last_price: 0.0,
            total_volume: 0.0,
            trade_count: 0,
            price_changes: Vec::new(),
            anomalies_detected: 0,
        }
    }
    
    fn add_data_point(&mut self, point: MarketDataPoint, config: &MonitorConfig) -> Option<String> {
        let mut alert = None;
        
        // 检测价格异常
        if self.last_price > 0.0 {
            let price_change_pct = ((point.price - self.last_price) / self.last_price * 100.0).abs();
            if price_change_pct > config.price_change_threshold {
                alert = Some(format!(
                    "⚠️ 价格异常警报！{} {} 价格变化 {:.2}% (${:.2} -> ${:.2})",
                    point.exchange, self.symbol, price_change_pct, self.last_price, point.price
                ));
                self.anomalies_detected += 1;
            }
            self.price_changes.push(price_change_pct);
        }
        
        // 检测成交量异常
        if self.data_points.len() >= 10 {
            let avg_volume: f64 = self.data_points.iter()
                .rev()
                .take(10)
                .map(|p| p.volume)
                .sum::<f64>() / 10.0;
            
            if point.volume > avg_volume * config.volume_anomaly_multiplier {
                let volume_alert = format!(
                    "📊 成交量异常！{} {} 成交量 {:.4} (平均值的 {:.1}倍)",
                    point.exchange, self.symbol, point.volume, point.volume / avg_volume
                );
                if alert.is_none() {
                    alert = Some(volume_alert);
                } else {
                    alert = Some(format!("{}\n{}", alert.unwrap(), volume_alert));
                }
                self.anomalies_detected += 1;
            }
        }
        
        // 更新数据
        self.last_price = point.price;
        self.total_volume += point.volume;
        self.trade_count += 1;
        
        // 维护窗口大小
        if self.data_points.len() >= config.window_size {
            self.data_points.pop_front();
        }
        self.data_points.push_back(point);
        
        alert
    }
    
    fn get_statistics(&self) -> String {
        let avg_price_change = if !self.price_changes.is_empty() {
            self.price_changes.iter().sum::<f64>() / self.price_changes.len() as f64
        } else {
            0.0
        };
        
        let volatility = if self.price_changes.len() > 1 {
            let mean = avg_price_change;
            let variance = self.price_changes.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>() / self.price_changes.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        format!(
            "📈 {} - 价格: ${:.2} | 成交量: {:.4} | 交易数: {} | 波动率: {:.3}% | 异常: {}",
            self.symbol, self.last_price, self.total_volume, self.trade_count, volatility, self.anomalies_detected
        )
    }
}

/// 监控系统主类
struct MonitoringSystem {
    config: MonitorConfig,
    monitors: Arc<Mutex<HashMap<String, SymbolMonitor>>>,
    start_time: Instant,
    total_events: Arc<Mutex<u64>>,
}

impl MonitoringSystem {
    fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            monitors: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
            total_events: Arc::new(Mutex::new(0)),
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
        let monitor = monitors.entry(symbol.clone()).or_insert_with(|| SymbolMonitor::new(symbol));
        
        if let Some(alert) = monitor.add_data_point(point, &self.config) {
            warn!("{}", alert);
        }
        
        let mut total = self.total_events.lock().await;
        *total += 1;
    }
    
    async fn generate_report(&self) {
        let monitors = self.monitors.lock().await;
        let total_events = *self.total_events.lock().await;
        let elapsed = self.start_time.elapsed().as_secs();
        
        println!("\n================== 监控系统报告 ==================");
        println!("运行时间: {} 秒 | 总事件数: {} | 速率: {:.1} 事件/秒", 
                 elapsed, total_events, total_events as f64 / elapsed as f64);
        println!("--------------------------------------------------");
        
        for (_, monitor) in monitors.iter() {
            println!("{}", monitor.get_statistics());
        }
        
        println!("==================================================\n");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    init_logging();
    
    println!("\n🚀 启动加密货币实时监控系统");
    println!("===================================");
    println!("监控交易所: Binance, OKX, Bybit");
    println!("监控币种: BTC/USDT, ETH/USDT");
    println!("功能: 价格监控, 成交量分析, 异常检测");
    println!("===================================\n");
    
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
    
    // 合并流
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(|error| {
            error!("流错误: {:?}", error);
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
    
    loop {
        tokio::select! {
            _ = &mut timeout => {
                info!("\n监控时间结束");
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
    
    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_ansi(true)
        .compact()
        .init()
}