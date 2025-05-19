use {
    crate::{
        traits::{Storage, StorageFactory},
        Store,
        parquet_store::ParquetStore,
        postgres_store::PostgresStore,
    },
    anyhow::{Result, anyhow},
    async_trait::async_trait,
    std::sync::Arc,
    windexer_geyser::config::{StorageConfig, StorageType},
};

/// Factory for creating storage instances based on configuration
pub struct WindexerStorageFactory {
    config: StorageConfig,
}

impl WindexerStorageFactory {
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl StorageFactory for WindexerStorageFactory {
    async fn create_storage(&self) -> Result<Arc<dyn Storage>> {
        match self.config.storage_type {
            StorageType::RocksDB => {
                let path = match &self.config.rocksdb_path {
                    Some(path) => path.clone(),
                    None => return Err(anyhow!("RocksDB path not configured")),
                };
                
                let store_config = crate::StoreConfig {
                    path: path.into(),
                    max_open_files: 1000, // Default
                    cache_capacity: 100 * 1024 * 1024, // 100 MB default
                };
                
                let store = Store::open(store_config)?;
                Ok(Arc::new(store))
            },
            StorageType::Parquet => {
                let config = match &self.config.parquet {
                    Some(config) => config.clone(),
                    None => return Err(anyhow!("Parquet configuration not provided")),
                };
                
                let store = ParquetStore::new(config).await?;
                Ok(Arc::new(store))
            },
            StorageType::Postgres => {
                let config = match &self.config.postgres {
                    Some(config) => config.clone(),
                    None => return Err(anyhow!("PostgreSQL configuration not provided")),
                };
                
                let store = PostgresStore::new(config).await?;
                Ok(Arc::new(store))
            }
        }
    }
}

/// Factory for creating hot and cold storage implementations
/// Hot storage is optimized for write performance (e.g., RocksDB)
/// Cold storage is optimized for query performance (e.g., Parquet or PostgreSQL)
pub struct HotColdStorageFactory {
    hot_config: StorageConfig,
    cold_config: Option<StorageConfig>,
}

impl HotColdStorageFactory {
    pub fn new(hot_config: StorageConfig, cold_config: Option<StorageConfig>) -> Self {
        Self { 
            hot_config,
            cold_config,
        }
    }
    
    pub async fn create_hot_storage(&self) -> Result<Arc<dyn Storage>> {
        let factory = WindexerStorageFactory::new(self.hot_config.clone());
        factory.create_storage().await
    }
    
    pub async fn create_cold_storage(&self) -> Result<Option<Arc<dyn Storage>>> {
        match &self.cold_config {
            Some(config) => {
                let factory = WindexerStorageFactory::new(config.clone());
                let storage = factory.create_storage().await?;
                Ok(Some(storage))
            },
            None => Ok(None),
        }
    }
} 