use anyhow::Result;
use clap::Parser;
use fluvio::{Fluvio, FluvioConfig, Offset};
use futures::StreamExt;
use monitor_anomaly::{
    detector::AnomalyDetectorManager, PriceAnomalyConfig, TimeSeriesData, VolumeAnomalyConfig,
};
use monitor_api::{server::ApiServer, state::AppState};
use monitor_core::{
    engine::MonitorEngine, EventType, MarketDataType, MonitorConfig, MonitorEvent,
};
use monitor_notifier::{
    manager::NotificationManager, telegram::TelegramNotifier, email::EmailNotifier,
    Notification, NotificationConfig,
};
use monitor_trader::{
    executor::AutoTrader,
    risk::SimpleRiskManager,
    strategy::AnomalyBasedStrategy,
};
use std::{path::PathBuf, sync::Arc};
use tokio::{signal, sync::mpsc};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Disable API server
    #[arg(long)]
    no_api: bool,
    
    /// Disable auto trading
    #[arg(long)]
    no_trading: bool,
    
    /// Disable notifications
    #[arg(long)]
    no_notifications: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    init_logging(args.debug);
    
    info!("Starting Crypto Monitor Application");
    
    // Load configuration
    let config = load_config(&args.config).await?;
    
    // Initialize database
    let db_pool = init_database(&config).await?;
    
    // Initialize Fluvio
    let fluvio = init_fluvio(&config).await?;
    
    // Create shared application state
    let app_state = AppState::new(db_pool.clone(), fluvio.clone());
    
    // Initialize monitor engine
    let mut monitor_engine = MonitorEngine::new(config.clone()).await?;
    monitor_engine.start().await?;
    
    // Initialize anomaly detector
    let anomaly_manager = Arc::new(AnomalyDetectorManager::new(
        VolumeAnomalyConfig::default(),
        PriceAnomalyConfig::default(),
    ));
    
    // Initialize notification manager if enabled
    let notification_manager = if !args.no_notifications {
        Some(Arc::new(init_notifications(&config.notification).await?))
    } else {
        None
    };
    
    // Initialize auto trader if enabled
    let auto_trader = if !args.no_trading {
        Some(Arc::new(init_auto_trader(&config).await?))
    } else {
        None
    };
    
    // Start API server if enabled
    if !args.no_api {
        let api_state = app_state.clone();
        tokio::spawn(async move {
            let server = ApiServer::new(config.clone(), api_state).await.unwrap();
            if let Err(e) = server.run().await {
                error!("API server error: {}", e);
            }
        });
    }
    
    // Start event processing
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
    
    // Spawn Fluvio consumer task
    let consumer_handle = tokio::spawn(process_events(
        fluvio.clone(),
        config.clone(),
        anomaly_manager.clone(),
        notification_manager.clone(),
        auto_trader.clone(),
        app_state.clone(),
    ));
    
    // Set up graceful shutdown
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };
    
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };
    
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    
    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down...");
        }
        _ = shutdown_rx.recv() => {
            info!("Received shutdown signal...");
        }
    }
    
    // Graceful shutdown
    info!("Initiating graceful shutdown...");
    
    monitor_engine.stop().await?;
    consumer_handle.abort();
    
    info!("Crypto Monitor Application stopped");
    
    Ok(())
}

fn init_logging(debug: bool) {
    let env_filter = if debug {
        "debug"
    } else {
        "info"
    };
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| env_filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn load_config(path: &PathBuf) -> Result<MonitorConfig> {
    let config_str = tokio::fs::read_to_string(path).await?;
    let config: MonitorConfig = serde_yaml::from_str(&config_str)?;
    Ok(config)
}

async fn init_database(config: &MonitorConfig) -> Result<sqlx::PgPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .connect(&config.database.url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    info!("Database initialized successfully");
    Ok(pool)
}

async fn init_fluvio(config: &MonitorConfig) -> Result<Arc<Fluvio>> {
    let fluvio_config = FluvioConfig::new(&config.fluvio.endpoint);
    let fluvio = Fluvio::connect_with_config(&fluvio_config).await?;
    info!("Connected to Fluvio at {}", config.fluvio.endpoint);
    Ok(Arc::new(fluvio))
}

async fn init_notifications(config: &NotificationConfig) -> Result<NotificationManager> {
    let mut manager = NotificationManager::new();
    
    if config.telegram.enabled {
        manager.add_channel(Box::new(TelegramNotifier::new(config.telegram.clone())));
    }
    
    if config.email.enabled {
        manager.add_channel(Box::new(EmailNotifier::new(config.email.clone())));
    }
    
    info!("Notification manager initialized");
    Ok(manager)
}

async fn init_auto_trader(config: &MonitorConfig) -> Result<AutoTrader> {
    // This is a simplified initialization - in production you'd configure properly
    let strategy = Box::new(AnomalyBasedStrategy::new(config.monitoring.trading.clone()));
    let risk_manager = Box::new(SimpleRiskManager::new(config.monitoring.trading.clone()));
    
    // Create execution client based on config
    // This would need proper initialization with exchange credentials
    let execution_client = create_execution_client(config).await?;
    
    let trader = AutoTrader::new(
        config.monitoring.trading.clone(),
        strategy,
        risk_manager,
        execution_client,
        10000.0, // Initial portfolio value
    );
    
    info!("Auto trader initialized");
    Ok(trader)
}

async fn create_execution_client(
    config: &MonitorConfig,
) -> Result<Arc<dyn barter_execution::ExecutionClient>> {
    // This is a placeholder - you'd create actual execution clients here
    // based on the exchange configuration
    unimplemented!("Execution client creation not implemented")
}

async fn process_events(
    fluvio: Arc<Fluvio>,
    config: MonitorConfig,
    anomaly_manager: Arc<AnomalyDetectorManager>,
    notification_manager: Option<Arc<NotificationManager>>,
    auto_trader: Option<Arc<AutoTrader>>,
    app_state: AppState,
) {
    let topic = format!("{}.market.trades", config.fluvio.topic_prefix);
    
    let consumer = fluvio
        .partition_consumer(&topic, 0)
        .await
        .expect("Failed to create consumer");
    
    let mut stream = consumer
        .stream(Offset::end())
        .await
        .expect("Failed to create stream");
    
    info!("Started processing events from topic: {}", topic);
    
    while let Some(Ok(record)) = stream.next().await {
        let value = record.get_value().to_vec();
        
        match serde_json::from_slice::<MonitorEvent>(&value) {
            Ok(event) => {
                process_single_event(
                    event,
                    &anomaly_manager,
                    notification_manager.as_ref(),
                    auto_trader.as_ref(),
                    &app_state,
                )
                .await;
            }
            Err(e) => {
                error!("Failed to deserialize event: {}", e);
            }
        }
    }
}

async fn process_single_event(
    event: MonitorEvent,
    anomaly_manager: &Arc<AnomalyDetectorManager>,
    notification_manager: Option<&Arc<NotificationManager>>,
    auto_trader: Option<&Arc<AutoTrader>>,
    app_state: &AppState,
) {
    // Process market data for anomaly detection
    if let EventType::MarketData(MarketDataType::Trade) = &event.event_type {
        if let Ok(trade_data) = serde_json::from_value::<MarketTradeData>(event.data.clone()) {
            let ts_data = TimeSeriesData {
                timestamp: event.timestamp,
                value: trade_data.price,
            };
            
            let anomalies = anomaly_manager.process_data(
                &trade_data.symbol,
                &trade_data.exchange,
                &ts_data,
            );
            
            for anomaly in anomalies {
                info!("Anomaly detected: {:?}", anomaly);
                
                // Send notification
                if let Some(notifier) = notification_manager {
                    let notification = Notification::from_anomaly(&anomaly);
                    if let Err(e) = notifier.send_all(&notification).await {
                        error!("Failed to send notification: {}", e);
                    }
                }
                
                // Process for auto trading
                if let Some(trader) = auto_trader {
                    if let Err(e) = trader.process_anomaly(&anomaly).await {
                        error!("Failed to process anomaly for trading: {}", e);
                    }
                }
                
                // Broadcast to WebSocket clients
                monitor_api::websocket::broadcast_anomaly_event(app_state, &anomaly);
            }
            
            // Update positions with current price
            if let Some(trader) = auto_trader {
                if let Err(e) = trader
                    .update_positions(&trade_data.symbol, &trade_data.exchange, trade_data.price)
                    .await
                {
                    error!("Failed to update positions: {}", e);
                }
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct MarketTradeData {
    symbol: String,
    exchange: String,
    price: f64,
    volume: f64,
}