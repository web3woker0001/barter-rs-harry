use crate::{MonitorError, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::info;

pub struct StorageManager {
    pool: PgPool,
}

impl StorageManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        
        info!("Database connection established");
        
        Ok(Self { pool })
    }
    
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        
        info!("Database migrations completed");
        Ok(())
    }
    
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }
}