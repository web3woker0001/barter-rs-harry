use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use monitor_core::MonitorEvent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub msg_type: WsMessageType,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WsMessageType {
    Subscribe,
    Unsubscribe,
    MarketData,
    Anomaly,
    Alert,
    Trade,
    Heartbeat,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
    pub channel: String,
    pub symbols: Vec<String>,
    pub exchanges: Vec<String>,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
    
    // Add client to connected clients
    let client_id = uuid::Uuid::new_v4();
    state.add_websocket_client(client_id, tx.clone());
    
    info!("WebSocket client connected: {}", client_id);
    
    // Spawn task to handle sending messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };
            
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });
    
    // Spawn task to handle receiving messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        handle_client_message(ws_msg, &state, client_id).await;
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });
    
    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
    
    // Remove client from connected clients
    state.remove_websocket_client(client_id);
    info!("WebSocket client disconnected: {}", client_id);
}

async fn handle_client_message(msg: WsMessage, state: &AppState, client_id: uuid::Uuid) {
    match msg.msg_type {
        WsMessageType::Subscribe => {
            if let Ok(sub) = serde_json::from_value::<Subscription>(msg.data) {
                state.add_subscription(client_id, sub);
                info!("Client {} subscribed to channels", client_id);
            }
        }
        WsMessageType::Unsubscribe => {
            if let Ok(sub) = serde_json::from_value::<Subscription>(msg.data) {
                state.remove_subscription(client_id, &sub);
                info!("Client {} unsubscribed from channels", client_id);
            }
        }
        WsMessageType::Heartbeat => {
            // Echo heartbeat back
            if let Some(tx) = state.get_websocket_client(client_id) {
                let _ = tx.send(WsMessage {
                    msg_type: WsMessageType::Heartbeat,
                    data: serde_json::json!({"timestamp": chrono::Utc::now()}),
                });
            }
        }
        _ => {
            warn!("Unexpected message type from client: {:?}", msg.msg_type);
        }
    }
}

pub fn broadcast_market_event(state: &AppState, event: &MonitorEvent) {
    let msg = WsMessage {
        msg_type: WsMessageType::MarketData,
        data: serde_json::to_value(event).unwrap_or_default(),
    };
    
    state.broadcast_to_subscribers(&msg, |sub| {
        // Check if client is subscribed to this event
        // This is a simplified version - you'd want more sophisticated filtering
        true
    });
}

pub fn broadcast_anomaly_event(state: &AppState, anomaly: &monitor_anomaly::AnomalyDetection) {
    let msg = WsMessage {
        msg_type: WsMessageType::Anomaly,
        data: serde_json::to_value(anomaly).unwrap_or_default(),
    };
    
    state.broadcast_to_subscribers(&msg, |sub| {
        sub.channel == "anomalies" || sub.channel == "all"
    });
}

pub fn broadcast_alert(state: &AppState, alert: serde_json::Value) {
    let msg = WsMessage {
        msg_type: WsMessageType::Alert,
        data: alert,
    };
    
    state.broadcast_to_subscribers(&msg, |sub| {
        sub.channel == "alerts" || sub.channel == "all"
    });
}