use crate::{MonitorConfig, MonitorError, MonitorEvent, Result, ExchangeConfig};
use barter::{
    engine::{Engine, EngineConfig},
    EngineEvent,
};
use barter_data::{
    event::MarketEvent,
    streams::{builder::StreamBuilder, reconnect::stream::ReconnectingStream},
    subscription::{
        book::{OrderBookL1, OrderBookL2},
        trade::PublicTrades,
    },
};
use barter_execution::ExecutionClient;
use barter_instrument::InstrumentIndex;
use fluvio::{Fluvio, FluvioConfig, Offset, RecordKey, TopicProducer};
use futures::{Stream, StreamExt};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct MonitorEngine {
    config: Arc<MonitorConfig>,
    fluvio: Arc<Fluvio>,
    producers: Arc<RwLock<HashMap<String, Arc<TopicProducer>>>>,
    engine_handle: Option<tokio::task::JoinHandle<()>>,
    event_tx: mpsc::UnboundedSender<MonitorEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<MonitorEvent>>,
}

impl MonitorEngine {
    pub async fn new(config: MonitorConfig) -> Result<Self> {
        let fluvio_config = FluvioConfig::new(&config.fluvio.endpoint);
        let fluvio = Fluvio::connect_with_config(&fluvio_config).await?;
        
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config: Arc::new(config),
            fluvio: Arc::new(fluvio),
            producers: Arc::new(RwLock::new(HashMap::new())),
            engine_handle: None,
            event_tx,
            event_rx: Some(event_rx),
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting monitor engine...");
        
        // Initialize Fluvio topics
        self.initialize_topics().await?;
        
        // Start market data collection
        self.start_market_data_collection().await?;
        
        // Start event processing
        self.start_event_processing().await?;
        
        info!("Monitor engine started successfully");
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping monitor engine...");
        
        if let Some(handle) = self.engine_handle.take() {
            handle.abort();
        }
        
        info!("Monitor engine stopped");
        Ok(())
    }
    
    async fn initialize_topics(&self) -> Result<()> {
        let admin = fluvio::FluvioAdmin::connect().await?;
        
        let topics = vec![
            format!("{}.market.trades", self.config.fluvio.topic_prefix),
            format!("{}.market.orderbook", self.config.fluvio.topic_prefix),
            format!("{}.market.candles", self.config.fluvio.topic_prefix),
            format!("{}.anomalies", self.config.fluvio.topic_prefix),
            format!("{}.alerts", self.config.fluvio.topic_prefix),
            format!("{}.trades", self.config.fluvio.topic_prefix),
        ];
        
        for topic in topics {
            match admin
                .create(
                    topic.clone(),
                    false,
                    fluvio::metadata::topic::TopicSpec::new_computed(
                        self.config.fluvio.partitions as i32,
                        self.config.fluvio.replication_factor as i32,
                        None,
                    ),
                )
                .await
            {
                Ok(_) => info!("Created topic: {}", topic),
                Err(e) => {
                    if e.to_string().contains("already exists") {
                        info!("Topic already exists: {}", topic);
                    } else {
                        return Err(MonitorError::Fluvio(e));
                    }
                }
            }
            
            let producer = self.fluvio.topic_producer(&topic).await?;
            self.producers.write().insert(topic, Arc::new(producer));
        }
        
        Ok(())
    }
    
    async fn start_market_data_collection(&mut self) -> Result<()> {
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        
        for exchange_config in &config.exchanges {
            if !exchange_config.enabled {
                continue;
            }
            
            info!("Starting data collection for {}", exchange_config.name);
            
            // Build streams for each exchange
            let streams = self.build_exchange_streams(&exchange_config).await?;
            
            // Spawn task to handle streams
            let exchange_name = exchange_config.name.clone();
            let tx = event_tx.clone();
            
            tokio::spawn(async move {
                Self::process_exchange_streams(exchange_name, streams, tx).await;
            });
        }
        
        Ok(())
    }
    
    async fn build_exchange_streams(
        &self,
        config: &ExchangeConfig,
    ) -> Result<Vec<Box<dyn Stream<Item = MarketEvent<String, PublicTrades>> + Send + Unpin>>> {
        // Simplified implementation - in production this would build actual streams
        // For now, return empty vec to allow compilation
        Ok(Vec::new())
    }
    
    async fn process_exchange_streams(
        exchange: String,
        mut streams: Vec<Box<dyn Stream<Item = MarketEvent<String, PublicTrades>> + Send + Unpin>>,
        tx: mpsc::UnboundedSender<MonitorEvent>,
    ) {
        use futures::stream::select_all;
        
        let mut combined = select_all(streams);
        
        while let Some(market_event) = combined.next().await {
            let monitor_event = MonitorEvent {
                id: uuid::Uuid::new_v4(),
                timestamp: market_event.time_received,
                source: crate::EventSource::Exchange(exchange.clone()),
                event_type: crate::EventType::MarketData(crate::MarketDataType::Trade),
                data: serde_json::to_value(&market_event).unwrap_or_default(),
            };
            
            if let Err(e) = tx.send(monitor_event) {
                error!("Failed to send market event: {}", e);
            }
        }
    }
    
    async fn start_event_processing(&mut self) -> Result<()> {
        let mut event_rx = self.event_rx.take().ok_or_else(|| {
            MonitorError::Other("Event receiver already taken".to_string())
        })?;
        
        let producers = self.producers.clone();
        let config = self.config.clone();
        
        self.engine_handle = Some(tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                Self::process_event(event, &producers, &config).await;
            }
        }));
        
        Ok(())
    }
    
    async fn process_event(
        event: MonitorEvent,
        producers: &Arc<RwLock<HashMap<String, Arc<TopicProducer>>>>,
        config: &MonitorConfig,
    ) {
        // Determine topic based on event type
        let topic = match &event.event_type {
            crate::EventType::MarketData(data_type) => match data_type {
                crate::MarketDataType::Trade => {
                    format!("{}.market.trades", config.fluvio.topic_prefix)
                }
                crate::MarketDataType::OrderBook => {
                    format!("{}.market.orderbook", config.fluvio.topic_prefix)
                }
                crate::MarketDataType::Candle => {
                    format!("{}.market.candles", config.fluvio.topic_prefix)
                }
                _ => return,
            },
            crate::EventType::Anomaly(_) => {
                format!("{}.anomalies", config.fluvio.topic_prefix)
            }
            crate::EventType::Alert(_) => {
                format!("{}.alerts", config.fluvio.topic_prefix)
            }
            crate::EventType::Trade(_) => {
                format!("{}.trades", config.fluvio.topic_prefix)
            }
            _ => return,
        };
        
        // Send to Fluvio
        if let Some(producer) = producers.read().get(&topic) {
            let data = match serde_json::to_string(&event) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    return;
                }
            };
            
            if let Err(e) = producer.send(RecordKey::NULL, data).await {
                error!("Failed to send event to Fluvio: {}", e);
            }
        }
    }
    
    pub fn get_event_sender(&self) -> mpsc::UnboundedSender<MonitorEvent> {
        self.event_tx.clone()
    }
}