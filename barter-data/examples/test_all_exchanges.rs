use barter_data::{
    event::DataKind,
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
        okx::Okx,
    },
    streams::{Streams, consumer::MarketStreamResult, reconnect::stream::ReconnectingStream},
    subscription::{
        book::{OrderBooksL1},
        trade::PublicTrades,
    },
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();
    
    info!("========================================");
    info!("Barter-RS Multi-Exchange Monitoring Test");
    info!("========================================");
    info!("Testing exchanges: Binance, OKX, Bybit");
    info!("Testing pairs: BTC/USDT, ETH/USDT");
    info!("Testing data: Trades, OrderBook L1");
    info!("Test duration: 30 seconds");
    info!("========================================\n");
    
    // Track statistics
    let mut stats = std::collections::HashMap::new();
    stats.insert("binance_spot_trades", 0u64);
    stats.insert("binance_futures_trades", 0u64);
    stats.insert("binance_spot_orderbook", 0u64);
    stats.insert("binance_futures_orderbook", 0u64);
    stats.insert("okx_spot_trades", 0u64);
    stats.insert("okx_futures_trades", 0u64);
    stats.insert("okx_spot_orderbook", 0u64);
    stats.insert("okx_futures_orderbook", 0u64);
    stats.insert("bybit_spot_trades", 0u64);
    stats.insert("bybit_futures_trades", 0u64);
    stats.insert("bybit_spot_orderbook", 0u64);
    stats.insert("bybit_futures_orderbook", 0u64);
    
    let mut last_prices = std::collections::HashMap::new();
    let mut total_events = 0u64;
    
    // Build streams for all exchanges
    info!("Initializing market data streams...");
    
    let streams: Streams<MarketStreamResult<MarketDataInstrument, DataKind>> = Streams::builder_multi()
        // Binance Spot & Futures - Trades
        .add(Streams::<PublicTrades>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
                (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
                (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            ])
        )
        
        // OKX Spot & Futures - Trades
        .add(Streams::<PublicTrades>::builder()
            .subscribe([
                (Okx, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
                (Okx, "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (Okx, "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
                (Okx, "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            ])
        )
        
        // Bybit Spot & Futures - Trades
        .add(Streams::<PublicTrades>::builder()
            .subscribe([
                (BybitSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
                (BybitSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (BybitPerpetualsUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
                (BybitPerpetualsUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            ])
        )
        
        // Binance OrderBook L1
        .add(Streams::<OrderBooksL1>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
                (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
                (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            ])
        )
        
        // OKX doesn't support OrderBook L1 in this configuration, skipping
        
        // Bybit OrderBook L1
        .add(Streams::<OrderBooksL1>::builder()
            .subscribe([
                (BybitSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
                (BybitSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
            ])
            .subscribe([
                (BybitPerpetualsUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
                (BybitPerpetualsUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            ])
        )
        
        .init()
        .await?;
    
    info!("Streams initialized successfully!");
    info!("Starting data collection...\n");
    
    // Create merged stream
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(|error| {
            warn!(?error, "Stream error occurred");
        });
    
    // Run test for 30 seconds
    let test_duration = Duration::from_secs(30);
    let timeout = tokio::time::sleep(test_duration);
    tokio::pin!(timeout);
    
    let start_time = std::time::Instant::now();
    
    loop {
        tokio::select! {
            _ = &mut timeout => {
                info!("\n\nTest duration completed!");
                break;
            }
            event = joined_stream.next() => {
                if let Some(event) = event {
                    match event {
                        barter_data::streams::reconnect::Event::Item(market_event) => {
                            total_events += 1;
                            
                            // Extract exchange info from the event
                            let exchange = format!("{:?}", market_event.exchange_time)
                                .split("::")
                                .last()
                                .unwrap_or("Unknown")
                                .to_lowercase();
                            
                            let market_type = match market_event.instrument.kind {
                                MarketDataInstrumentKind::Spot => "spot",
                                MarketDataInstrumentKind::Perpetual => "futures",
                                _ => "unknown",
                            };
                            
                            let symbol = format!("{}/{}",
                                market_event.instrument.base,
                                market_event.instrument.quote
                            );
                            
                            // Process event based on type
                            match market_event.kind {
                                DataKind::Trade(trade) => {
                                    let key = format!("{}_{}_trades", exchange, market_type);
                                    if let Some(count) = stats.get_mut(&key as &str) {
                                        *count += 1;
                                        
                                        // Store last price
                                        let price_key = format!("{}_{}_{}", exchange, market_type, symbol);
                                        last_prices.insert(price_key.clone(), trade.price);
                                        
                                        // Print every 50th trade
                                        if *count % 50 == 0 {
                                            info!("[{}] {} {} - Trade #{}: Price: ${:.2}, Volume: {:.4}",
                                                exchange.to_uppercase(),
                                                market_type.to_uppercase(),
                                                symbol,
                                                count,
                                                trade.price,
                                                trade.amount
                                            );
                                        }
                                    }
                                },
                                DataKind::OrderBookL1(book) => {
                                    let key = format!("{}_{}_orderbook", exchange, market_type);
                                    if let Some(count) = stats.get_mut(&key as &str) {
                                        *count += 1;
                                        
                                        // Print every 100th orderbook update
                                        if *count % 100 == 0 {
                                            if let (Some(bid), Some(ask)) = (book.best_bid, book.best_ask) {
                                                let spread = ask.price - bid.price;
                                                let spread_pct = (spread / bid.price) * 100.0;
                                                
                                                info!("[{}] {} {} - OrderBook #{}: Bid: ${:.2}, Ask: ${:.2}, Spread: ${:.2} ({:.3}%)",
                                                    exchange.to_uppercase(),
                                                    market_type.to_uppercase(),
                                                    symbol,
                                                    count,
                                                    bid.price,
                                                    ask.price,
                                                    spread,
                                                    spread_pct
                                                );
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }
                            
                            // Print progress every 1000 events
                            if total_events % 1000 == 0 {
                                let elapsed = start_time.elapsed().as_secs();
                                info!("\n=== Progress Update ===");
                                info!("Time elapsed: {} seconds", elapsed);
                                info!("Total events processed: {}", total_events);
                                info!("Events per second: {:.1}", total_events as f64 / elapsed as f64);
                                info!("====================\n");
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
    
    // Print final summary
    println!("\n\n========================================");
    println!("TEST RESULTS SUMMARY");
    println!("========================================");
    println!("Test duration: {} seconds", test_duration.as_secs());
    println!("Total events received: {}", total_events);
    println!("Average events/second: {:.1}", total_events as f64 / test_duration.as_secs() as f64);
    
    println!("\n--- BINANCE ---");
    println!("  Spot:");
    println!("    Trades: {}", stats.get("binance_spot_trades").unwrap_or(&0));
    println!("    OrderBook updates: {}", stats.get("binance_spot_orderbook").unwrap_or(&0));
    println!("  Futures:");
    println!("    Trades: {}", stats.get("binance_futures_trades").unwrap_or(&0));
    println!("    OrderBook updates: {}", stats.get("binance_futures_orderbook").unwrap_or(&0));
    
    println!("\n--- OKX ---");
    println!("  Spot:");
    println!("    Trades: {}", stats.get("okx_spot_trades").unwrap_or(&0));
    println!("    OrderBook updates: {}", stats.get("okx_spot_orderbook").unwrap_or(&0));
    println!("  Futures:");
    println!("    Trades: {}", stats.get("okx_futures_trades").unwrap_or(&0));
    println!("    OrderBook updates: {}", stats.get("okx_futures_orderbook").unwrap_or(&0));
    
    println!("\n--- BYBIT ---");
    println!("  Spot:");
    println!("    Trades: {}", stats.get("bybit_spot_trades").unwrap_or(&0));
    println!("    OrderBook updates: {}", stats.get("bybit_spot_orderbook").unwrap_or(&0));
    println!("  Futures:");
    println!("    Trades: {}", stats.get("bybit_futures_trades").unwrap_or(&0));
    println!("    OrderBook updates: {}", stats.get("bybit_futures_orderbook").unwrap_or(&0));
    
    // Check which exchanges worked
    println!("\n--- CONNECTION STATUS ---");
    let mut working_exchanges = Vec::new();
    let mut non_working = Vec::new();
    
    for exchange in ["binance", "okx", "bybit"] {
        let spot_trades = stats.get(&format!("{}_spot_trades", exchange) as &str).unwrap_or(&0);
        let futures_trades = stats.get(&format!("{}_futures_trades", exchange) as &str).unwrap_or(&0);
        let spot_ob = stats.get(&format!("{}_spot_orderbook", exchange) as &str).unwrap_or(&0);
        let futures_ob = stats.get(&format!("{}_futures_orderbook", exchange) as &str).unwrap_or(&0);
        
        if spot_trades + futures_trades + spot_ob + futures_ob > 0 {
            working_exchanges.push(exchange.to_uppercase());
        } else {
            non_working.push(exchange.to_uppercase());
        }
    }
    
    println!("✅ Working exchanges: {:?}", working_exchanges);
    if !non_working.is_empty() {
        println!("❌ Non-working exchanges: {:?}", non_working);
    }
    
    // Show last prices
    if !last_prices.is_empty() {
        println!("\n--- LAST PRICES ---");
        for (key, price) in last_prices.iter() {
            println!("  {}: ${:.2}", key, price);
        }
    }
    
    println!("\n========================================");
    println!("TEST COMPLETED SUCCESSFULLY!");
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