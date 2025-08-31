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
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();
    
    println!("\n========================================");
    println!("Barter-RS Exchange Monitoring Test");
    println!("========================================");
    println!("Testing: Binance, OKX, Bybit");
    println!("Pairs: BTC/USDT, ETH/USDT");
    println!("Data: Public Trades only");
    println!("Duration: 30 seconds");
    println!("========================================\n");
    
    // Track statistics
    let mut event_count = 0u64;
    let mut exchange_events = std::collections::HashMap::new();
    exchange_events.insert("binance", 0u64);
    exchange_events.insert("okx", 0u64);
    exchange_events.insert("bybit", 0u64);
    
    info!("Initializing streams...");
    
    // Build streams for all exchanges - PublicTrades only
    let streams = Streams::<PublicTrades>::builder()
        // Binance Spot
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])
        // Binance Futures
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])
        // OKX (both spot and futures)
        .subscribe([
            (Okx, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (Okx, "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (Okx, "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            (Okx, "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])
        // Bybit Spot
        .subscribe([
            (BybitSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (BybitSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])
        // Bybit Futures
        .subscribe([
            (BybitPerpetualsUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            (BybitPerpetualsUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])
        .init()
        .await?;
    
    info!("Streams initialized successfully!");
    info!("Starting data collection...\n");
    
    // Create merged stream
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(|error| {
            warn!(?error, "Stream error");
        });
    
    // Run test for 30 seconds
    let test_duration = Duration::from_secs(30);
    let timeout = tokio::time::sleep(test_duration);
    tokio::pin!(timeout);
    
    let start_time = std::time::Instant::now();
    
    loop {
        tokio::select! {
            _ = &mut timeout => {
                info!("\nTest duration completed!");
                break;
            }
            event = joined_stream.next() => {
                if let Some(event) = event {
                    match event {
                        barter_data::streams::reconnect::Event::Item(market_event) => {
                            event_count += 1;
                            
                            // Extract exchange name from debug output
                            let debug_str = format!("{:?}", market_event);
                            
                            // Count events by exchange
                            if debug_str.contains("Binance") {
                                *exchange_events.get_mut("binance").unwrap() += 1;
                            } else if debug_str.contains("Okx") {
                                *exchange_events.get_mut("okx").unwrap() += 1;
                            } else if debug_str.contains("Bybit") {
                                *exchange_events.get_mut("bybit").unwrap() += 1;
                            }
                            
                            // Print sample trades every 100 events
                            if event_count % 100 == 0 {
                                let elapsed = start_time.elapsed().as_secs();
                                info!("Events: {} | Time: {}s | Rate: {:.1}/s",
                                    event_count,
                                    elapsed,
                                    event_count as f64 / elapsed as f64
                                );
                                
                                // Print a sample trade (market_event.kind is already a PublicTrade)
                                info!("Sample Trade - Price: ${:.2}, Amount: {:.4}, Side: {:?}",
                                    market_event.kind.price,
                                    market_event.kind.amount,
                                    market_event.kind.side
                                );
                            }
                        },
                        barter_data::streams::reconnect::Event::Reconnecting(exchange_id) => {
                            warn!("Exchange reconnecting: {:?}", exchange_id);
                        }
                    }
                }
            }
        }
    }
    
    // Print summary
    println!("\n========================================");
    println!("TEST RESULTS SUMMARY");
    println!("========================================");
    println!("Test duration: {} seconds", test_duration.as_secs());
    println!("Total events received: {}", event_count);
    println!("Average events/second: {:.1}", event_count as f64 / test_duration.as_secs() as f64);
    
    println!("\n--- Events by Exchange ---");
    for (exchange, count) in &exchange_events {
        println!("{}: {} events", exchange.to_uppercase(), count);
        if *count > 0 {
            println!("  ✅ Connection successful");
        } else {
            println!("  ❌ No data received");
        }
    }
    
    // Determine which exchanges worked
    let working: Vec<_> = exchange_events
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(name, _)| name.to_uppercase())
        .collect();
    
    let not_working: Vec<_> = exchange_events
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| name.to_uppercase())
        .collect();
    
    println!("\n--- Connection Status ---");
    if !working.is_empty() {
        println!("✅ Working exchanges: {}", working.join(", "));
    }
    if !not_working.is_empty() {
        println!("❌ Non-working exchanges: {}", not_working.join(", "));
    }
    
    println!("\n========================================");
    if event_count > 0 {
        println!("TEST COMPLETED SUCCESSFULLY!");
    } else {
        println!("TEST COMPLETED - NO DATA RECEIVED");
    }
    println!("========================================\n");
    
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