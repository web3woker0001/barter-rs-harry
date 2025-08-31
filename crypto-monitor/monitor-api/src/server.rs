use crate::{handlers, state::AppState, websocket};
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use monitor_core::{MonitorConfig, Result};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub struct ApiServer {
    app: Router,
    addr: SocketAddr,
}

impl ApiServer {
    pub async fn new(config: MonitorConfig, state: AppState) -> Result<Self> {
        let app = Router::new()
            // Health check
            .route("/health", get(handlers::health_check))
            
            // System status
            .route("/api/v1/status", get(handlers::get_system_status))
            
            // Market data endpoints
            .route("/api/v1/market/stats", get(handlers::get_market_stats))
            .route("/api/v1/market/history", get(handlers::get_market_history))
            .route("/api/v1/market/orderbook", get(handlers::get_orderbook))
            
            // Anomaly endpoints
            .route("/api/v1/anomalies", get(handlers::get_anomalies))
            .route("/api/v1/anomalies/stats", get(handlers::get_anomaly_stats))
            
            // Trading endpoints
            .route("/api/v1/trading/config", get(handlers::get_trading_config))
            .route("/api/v1/trading/config", post(handlers::update_trading_config))
            .route("/api/v1/trading/positions", get(handlers::get_positions))
            .route("/api/v1/trading/orders", get(handlers::get_orders))
            .route("/api/v1/trading/orders", post(handlers::place_order))
            .route("/api/v1/trading/orders/:id", delete(handlers::cancel_order))
            
            // Alert configuration
            .route("/api/v1/alerts/config", get(handlers::get_alert_config))
            .route("/api/v1/alerts/config", post(handlers::update_alert_config))
            .route("/api/v1/alerts/history", get(handlers::get_alert_history))
            
            // WebSocket endpoint for real-time data
            .route("/ws", get(websocket::websocket_handler))
            
            // Add state
            .with_state(state)
            
            // Add CORS middleware
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            );
        
        let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
        
        Ok(Self { app, addr })
    }
    
    pub async fn run(self) -> Result<()> {
        info!("API server listening on {}", self.addr);
        
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| monitor_core::MonitorError::Other(e.to_string()))?;
            
        axum::serve(listener, self.app)
            .await
            .map_err(|e| monitor_core::MonitorError::Other(e.to_string()))?;
            
        Ok(())
    }
}