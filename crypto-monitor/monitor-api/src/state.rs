use crate::websocket::Subscription;
use dashmap::DashMap;
use fluvio::Fluvio;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub fluvio: Arc<Fluvio>,
    pub websocket_clients: Arc<DashMap<Uuid, mpsc::UnboundedSender<crate::websocket::WsMessage>>>,
    pub subscriptions: Arc<DashMap<Uuid, Vec<Subscription>>>,
}

impl AppState {
    pub fn new(db: PgPool, fluvio: Arc<Fluvio>) -> Self {
        Self {
            db,
            fluvio,
            websocket_clients: Arc::new(DashMap::new()),
            subscriptions: Arc::new(DashMap::new()),
        }
    }
    
    pub fn add_websocket_client(
        &self,
        client_id: Uuid,
        tx: mpsc::UnboundedSender<crate::websocket::WsMessage>,
    ) {
        self.websocket_clients.insert(client_id, tx);
    }
    
    pub fn remove_websocket_client(&self, client_id: Uuid) {
        self.websocket_clients.remove(&client_id);
        self.subscriptions.remove(&client_id);
    }
    
    pub fn get_websocket_client(
        &self,
        client_id: Uuid,
    ) -> Option<mpsc::UnboundedSender<crate::websocket::WsMessage>> {
        self.websocket_clients.get(&client_id).map(|c| c.clone())
    }
    
    pub fn add_subscription(&self, client_id: Uuid, subscription: Subscription) {
        self.subscriptions
            .entry(client_id)
            .or_insert_with(Vec::new)
            .push(subscription);
    }
    
    pub fn remove_subscription(&self, client_id: Uuid, subscription: &Subscription) {
        if let Some(mut subs) = self.subscriptions.get_mut(&client_id) {
            subs.retain(|s| s.channel != subscription.channel);
        }
    }
    
    pub fn broadcast_to_subscribers<F>(&self, message: &crate::websocket::WsMessage, filter: F)
    where
        F: Fn(&Subscription) -> bool,
    {
        for entry in self.subscriptions.iter() {
            let client_id = entry.key();
            let subs = entry.value();
            
            if subs.iter().any(&filter) {
                if let Some(tx) = self.websocket_clients.get(client_id) {
                    let _ = tx.send(message.clone());
                }
            }
        }
    }
}