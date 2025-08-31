use crate::{MonitorError, MonitorEvent, Result};
use fluvio::{Fluvio, Offset, RecordKey};
use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct EventStream {
    fluvio: Fluvio,
    topic: String,
    tx: mpsc::UnboundedSender<MonitorEvent>,
}

impl EventStream {
    pub async fn new(
        fluvio: Fluvio,
        topic: String,
        tx: mpsc::UnboundedSender<MonitorEvent>,
    ) -> Result<Self> {
        Ok(Self { fluvio, topic, tx })
    }
    
    pub async fn start_consuming(&self) -> Result<()> {
        let consumer = self.fluvio
            .partition_consumer(&self.topic, 0)
            .await?;
        
        let mut stream = consumer.stream(Offset::end()).await?;
        
        info!("Started consuming from topic: {}", self.topic);
        
        while let Some(Ok(record)) = stream.next().await {
            let value = record.get_value().to_vec();
            
            match serde_json::from_slice::<MonitorEvent>(&value) {
                Ok(event) => {
                    if let Err(e) = self.tx.send(event) {
                        error!("Failed to send event: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to deserialize event: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn publish(&self, event: &MonitorEvent) -> Result<()> {
        let data = serde_json::to_string(event)?;
        
        let producer = self.fluvio.topic_producer(&self.topic).await?;
        producer.send(RecordKey::NULL, data).await?;
        
        Ok(())
    }
}