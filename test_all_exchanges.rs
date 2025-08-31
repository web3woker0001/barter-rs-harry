use barter_data::{
    event::DataKind,
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
        okx::Okx,
    },
    streams::{Streams, consumer::MarketStreamResult, reconnect::stream::ReconnectingStream},
    subscription::{
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExchangeStats {
    exchange: String,
    symbol: String,
    market_type: String,
    trades_count: u64,
    orderbook_updates: u64,
    last_price: Option<f64>,
    last_bid: Option<f64>,
    last_ask: Option<f64>,
    total_volume: f64,
    first_update_time: Option<DateTime<Utc>>,
    last_update_time: Option<DateTime<Utc>>,
    errors: Vec<String>,
}

impl ExchangeStats {
    fn new(exchange: String, symbol: String, market_type: String) -> Self {
        Self {
            exchange,
            symbol,
            market_type,
            trades_count: 0,
            orderbook_updates: 0,
            last_price: None,
            last_bid: None,
            last_ask: None,
            total_volume: 0.0,
            first_update_time: None,
            last_update_time: None,
            errors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestResults {
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    duration_seconds: i64,
    total_events_received: u64,
    exchanges_tested: Vec<String>,
    symbols_tested: Vec<String>,
    stats_by_exchange: HashMap<String, Vec<ExchangeStats>>,
    connection_status: HashMap<String, bool>,
    summary: TestSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSummary {
    total_exchanges_tested: usize,
    successful_connections: usize,
    failed_connections: usize,
    total_trades_received: u64,
    total_orderbook_updates: u64,
    exchanges_with_data: Vec<String>,
    exchanges_without_data: Vec<String>,
}

async fn test_all_exchanges() -> Result<TestResults, Box<dyn std::error::Error>> {
    let start_time = Utc::now();
    let stats = Arc::new(Mutex::new(HashMap::<String, ExchangeStats>::new()));
    let total_events = Arc::new(Mutex::new(0u64));
    
    // Test symbols - using popular pairs that are available on all exchanges
    let test_symbols = vec![
        ("btc", "usdt"),  // Bitcoin
        ("eth", "usdt"),  // Ethereum
        ("bnb", "usdt"),  // Binance Coin (might not be on all exchanges)
    ];
    
    info!("Starting comprehensive exchange monitoring test");
    info!("Testing exchanges: Binance, OKX, Bybit");
    info!("Testing symbols: BTC/USDT, ETH/USDT, BNB/USDT");
    info!("Testing data types: Trades, OrderBook L1, OrderBook L2");
    
    // Initialize statistics for all exchange-symbol combinations
    {
        let mut stats_guard = stats.lock().await;
        for (base, quote) in &test_symbols {
            let symbol = format!("{}/{}", base.to_uppercase(), quote.to_uppercase());
            
            // Binance Spot
            let key = format!("binance_spot_{}", symbol);
            stats_guard.insert(key, ExchangeStats::new("Binance".to_string(), symbol.clone(), "Spot".to_string()));
            
            // Binance Futures
            let key = format!("binance_futures_{}", symbol);
            stats_guard.insert(key, ExchangeStats::new("Binance".to_string(), symbol.clone(), "Futures".to_string()));
            
            // OKX Spot
            let key = format!("okx_spot_{}", symbol);
            stats_guard.insert(key, ExchangeStats::new("OKX".to_string(), symbol.clone(), "Spot".to_string()));
            
            // OKX Futures
            let key = format!("okx_futures_{}", symbol);
            stats_guard.insert(key, ExchangeStats::new("OKX".to_string(), symbol.clone(), "Perpetual".to_string()));
            
            // Bybit Spot
            let key = format!("bybit_spot_{}", symbol);
            stats_guard.insert(key, ExchangeStats::new("Bybit".to_string(), symbol.clone(), "Spot".to_string()));
            
            // Bybit Futures
            let key = format!("bybit_futures_{}", symbol);
            stats_guard.insert(key, ExchangeStats::new("Bybit".to_string(), symbol.clone(), "Futures".to_string()));
        }
    }
    
    // Build multi-exchange streams
    let streams_result = build_multi_exchange_streams(&test_symbols).await;
    
    if let Err(e) = streams_result {
        error!("Failed to initialize streams: {}", e);
        return Err(e.into());
    }
    
    let streams = streams_result.unwrap();
    
    // Create merged stream
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(|error| {
            warn!(?error, "MarketStream generated error");
        });
    
    // Run test for specified duration
    let test_duration = Duration::from_secs(30); // Run for 30 seconds
    let timeout = tokio::time::sleep(test_duration);
    tokio::pin!(timeout);
    
    info!("Starting data collection for {} seconds...", test_duration.as_secs());
    
    loop {
        tokio::select! {
            _ = &mut timeout => {
                info!("Test duration completed");
                break;
            }
            event = joined_stream.next() => {
                if let Some(event) = event {
                    // Update statistics
                    let stats_clone = Arc::clone(&stats);
                    let total_events_clone = Arc::clone(&total_events);
                    
                    tokio::spawn(async move {
                        process_event(event, stats_clone, total_events_clone).await;
                    });
                }
            }
        }
    }
    
    let end_time = Utc::now();
    let duration = end_time.signed_duration_since(start_time);
    
    // Prepare results
    let stats_guard = stats.lock().await;
    let total_events_count = *total_events.lock().await;
    
    let mut stats_by_exchange: HashMap<String, Vec<ExchangeStats>> = HashMap::new();
    let mut connection_status: HashMap<String, bool> = HashMap::new();
    let mut total_trades = 0u64;
    let mut total_orderbook_updates = 0u64;
    let mut exchanges_with_data = Vec::new();
    let mut exchanges_without_data = Vec::new();
    
    // Organize stats by exchange
    for (_, stat) in stats_guard.iter() {
        let exchange_key = stat.exchange.clone();
        stats_by_exchange.entry(exchange_key.clone()).or_insert_with(Vec::new).push(stat.clone());
        
        total_trades += stat.trades_count;
        total_orderbook_updates += stat.orderbook_updates;
        
        let has_data = stat.trades_count > 0 || stat.orderbook_updates > 0;
        connection_status.insert(format!("{}_{}", stat.exchange, stat.market_type), has_data);
        
        if has_data && !exchanges_with_data.contains(&exchange_key) {
            exchanges_with_data.push(exchange_key);
        }
    }
    
    // Find exchanges without data
    for exchange in vec!["Binance", "OKX", "Bybit"] {
        if !exchanges_with_data.contains(&exchange.to_string()) {
            exchanges_without_data.push(exchange.to_string());
        }
    }
    
    let summary = TestSummary {
        total_exchanges_tested: 3,
        successful_connections: exchanges_with_data.len(),
        failed_connections: exchanges_without_data.len(),
        total_trades_received: total_trades,
        total_orderbook_updates: total_orderbook_updates,
        exchanges_with_data,
        exchanges_without_data,
    };
    
    let results = TestResults {
        start_time,
        end_time,
        duration_seconds: duration.num_seconds(),
        total_events_received: total_events_count,
        exchanges_tested: vec!["Binance".to_string(), "OKX".to_string(), "Bybit".to_string()],
        symbols_tested: test_symbols.iter().map(|(b, q)| format!("{}/{}", b.to_uppercase(), q.to_uppercase())).collect(),
        stats_by_exchange,
        connection_status,
        summary,
    };
    
    Ok(results)
}

async fn build_multi_exchange_streams(
    test_symbols: &Vec<(&str, &str)>
) -> Result<Streams<MarketStreamResult<MarketDataInstrument, DataKind>>, Box<dyn std::error::Error>> {
    
    let mut builder = Streams::builder_multi();
    
    // Add PublicTrades streams
    let mut trades_builder = Streams::<PublicTrades>::builder();
    
    for (base, quote) in test_symbols {
        // Binance
        trades_builder = trades_builder
            .subscribe([
                (BinanceSpot::default(), *base, *quote, MarketDataInstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), *base, *quote, MarketDataInstrumentKind::Perpetual, PublicTrades),
            ]);
        
        // OKX
        trades_builder = trades_builder
            .subscribe([
                (Okx, *base, *quote, MarketDataInstrumentKind::Spot, PublicTrades),
                (Okx, *base, *quote, MarketDataInstrumentKind::Perpetual, PublicTrades),
            ]);
        
        // Bybit
        trades_builder = trades_builder
            .subscribe([
                (BybitSpot::default(), *base, *quote, MarketDataInstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (BybitPerpetualsUsd::default(), *base, *quote, MarketDataInstrumentKind::Perpetual, PublicTrades),
            ]);
    }
    
    builder = builder.add(trades_builder);
    
    // Add OrderBooksL1 streams
    let mut l1_builder = Streams::<OrderBooksL1>::builder();
    
    for (base, quote) in test_symbols {
        // Binance
        l1_builder = l1_builder
            .subscribe([
                (BinanceSpot::default(), *base, *quote, MarketDataInstrumentKind::Spot, OrderBooksL1),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), *base, *quote, MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            ]);
        
        // OKX
        l1_builder = l1_builder
            .subscribe([
                (Okx, *base, *quote, MarketDataInstrumentKind::Spot, OrderBooksL1),
                (Okx, *base, *quote, MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            ]);
        
        // Bybit
        l1_builder = l1_builder
            .subscribe([
                (BybitSpot::default(), *base, *quote, MarketDataInstrumentKind::Spot, OrderBooksL1),
            ])
            .subscribe([
                (BybitPerpetualsUsd::default(), *base, *quote, MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            ]);
    }
    
    builder = builder.add(l1_builder);
    
    // Build and initialize streams
    let streams = builder
        .init()
        .await?;
    
    Ok(streams)
}

async fn process_event(
    event: MarketStreamResult<MarketDataInstrument, DataKind>,
    stats: Arc<Mutex<HashMap<String, ExchangeStats>>>,
    total_events: Arc<Mutex<u64>>,
) {
    // Increment total events
    {
        let mut total = total_events.lock().await;
        *total += 1;
    }
    
    match event {
        Ok(event) => {
            let mut stats_guard = stats.lock().await;
            
            // Build key for stats lookup
            let exchange_name = format!("{:?}", event.instrument.exchange_id).split("::").last().unwrap_or("Unknown").to_string();
            let market_type = match event.instrument.kind {
                MarketDataInstrumentKind::Spot => "spot",
                MarketDataInstrumentKind::Perpetual => "futures",
                _ => "unknown",
            };
            let symbol = format!("{}/{}", event.instrument.base, event.instrument.quote);
            let key = format!("{}_{}_{}",
                exchange_name.to_lowercase(),
                market_type,
                symbol
            );
            
            if let Some(stat) = stats_guard.get_mut(&key) {
                // Update timestamp
                if stat.first_update_time.is_none() {
                    stat.first_update_time = Some(event.time_exchange);
                }
                stat.last_update_time = Some(event.time_exchange);
                
                // Process different data types
                match event.kind {
                    DataKind::Trade(trade) => {
                        stat.trades_count += 1;
                        stat.last_price = Some(trade.price);
                        stat.total_volume += trade.quantity;
                        
                        if stat.trades_count % 100 == 0 {
                            info!(
                                "[{}] {} {} - Trades: {}, Last Price: {:.2}, Volume: {:.2}",
                                exchange_name,
                                market_type,
                                symbol,
                                stat.trades_count,
                                trade.price,
                                stat.total_volume
                            );
                        }
                    },
                    DataKind::OrderBookL1(book) => {
                        stat.orderbook_updates += 1;
                        stat.last_bid = Some(book.bid.price);
                        stat.last_ask = Some(book.ask.price);
                        
                        if stat.orderbook_updates % 100 == 0 {
                            info!(
                                "[{}] {} {} - OrderBook Updates: {}, Bid: {:.2}, Ask: {:.2}, Spread: {:.4}",
                                exchange_name,
                                market_type,
                                symbol,
                                stat.orderbook_updates,
                                book.bid.price,
                                book.ask.price,
                                book.ask.price - book.bid.price
                            );
                        }
                    },
                    DataKind::OrderBookL2(book) => {
                        stat.orderbook_updates += 1;
                        if let Some(best_bid) = book.bids.first() {
                            stat.last_bid = Some(best_bid.price);
                        }
                        if let Some(best_ask) = book.asks.first() {
                            stat.last_ask = Some(best_ask.price);
                        }
                    },
                    _ => {}
                }
            }
        },
        Err(e) => {
            error!("Stream error: {:?}", e);
        }
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_ansi(true)
        .json()
        .init()
}

fn save_results(results: &TestResults) -> Result<(), Box<dyn std::error::Error>> {
    // Save as JSON
    let json = serde_json::to_string_pretty(&results)?;
    let mut file = File::create("test_results.json")?;
    file.write_all(json.as_bytes())?;
    
    // Print summary
    println!("\n========== TEST RESULTS ==========");
    println!("Test Duration: {} seconds", results.duration_seconds);
    println!("Total Events Received: {}", results.total_events_received);
    println!("\n--- Exchange Summary ---");
    println!("Exchanges Tested: {}", results.summary.total_exchanges_tested);
    println!("Successful Connections: {}", results.summary.successful_connections);
    println!("Failed Connections: {}", results.summary.failed_connections);
    println!("\n--- Data Summary ---");
    println!("Total Trades: {}", results.summary.total_trades_received);
    println!("Total OrderBook Updates: {}", results.summary.total_orderbook_updates);
    println!("\n--- Exchange Status ---");
    println!("Working Exchanges: {:?}", results.summary.exchanges_with_data);
    println!("Non-Working Exchanges: {:?}", results.summary.exchanges_without_data);
    
    println!("\n--- Detailed Stats by Exchange ---");
    for (exchange, stats_list) in &results.stats_by_exchange {
        println!("\n{} Exchange:", exchange);
        for stat in stats_list {
            println!("  {} {}:", stat.market_type, stat.symbol);
            println!("    - Trades: {}", stat.trades_count);
            println!("    - OrderBook Updates: {}", stat.orderbook_updates);
            if let Some(price) = stat.last_price {
                println!("    - Last Price: {:.2}", price);
            }
            if let (Some(bid), Some(ask)) = (stat.last_bid, stat.last_ask) {
                println!("    - Bid/Ask: {:.2}/{:.2} (Spread: {:.4})", bid, ask, ask - bid);
            }
            if stat.total_volume > 0.0 {
                println!("    - Total Volume: {:.4}", stat.total_volume);
            }
        }
    }
    
    println!("\nDetailed results saved to: test_results.json");
    println!("===================================\n");
    
    Ok(())
}

#[tokio::main]
async fn main() {
    init_logging();
    
    info!("Starting Barter-RS Exchange Monitoring Test");
    info!("This test will monitor Binance, OKX, and Bybit exchanges");
    info!("Monitoring: Price feeds, Trading volume, Order book depth");
    
    match test_all_exchanges().await {
        Ok(results) => {
            info!("Test completed successfully!");
            if let Err(e) = save_results(&results) {
                error!("Failed to save results: {}", e);
            }
        },
        Err(e) => {
            error!("Test failed: {}", e);
        }
    }
}