use crate::{EventSource, EventType, MonitorEvent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct EventBuilder {
    id: Uuid,
    timestamp: DateTime<Utc>,
    source: Option<EventSource>,
    event_type: Option<EventType>,
    data: Option<serde_json::Value>,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source: None,
            event_type: None,
            data: None,
        }
    }
    
    pub fn with_source(mut self, source: EventSource) -> Self {
        self.source = Some(source);
        self
    }
    
    pub fn with_type(mut self, event_type: EventType) -> Self {
        self.event_type = Some(event_type);
        self
    }
    
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    
    pub fn build(self) -> Option<MonitorEvent> {
        Some(MonitorEvent {
            id: self.id,
            timestamp: self.timestamp,
            source: self.source?,
            event_type: self.event_type?,
            data: self.data.unwrap_or(serde_json::Value::Null),
        })
    }
}