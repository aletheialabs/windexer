use {
    crate::traits::Storage,
    anyhow::{Result, anyhow},
    std::sync::Arc,
    async_trait::async_trait,
    sqlx::{
        postgres::{PgPool, PgPoolOptions, PgRow},
        Row,
    },
    windexer_geyser::config::PostgresConfig,
    windexer_common::types::{
        AccountData,
        TransactionData,
        BlockData,
    },
};

/// PostgreSQL storage implementation
pub struct PostgresStore {
    config: PostgresConfig,
    pool: PgPool,
}

impl PostgresStore {
    pub async fn new(config: PostgresConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections as u32)
            .connect(&config.connection_string)
            .await?;
            
        let store = Self {
            config,
            pool,
        };
        
        // Initialize database schema if needed
        if config.create_tables {
            store.initialize_schema().await?;
        }
        
        Ok(store)
    }
    
    async fn initialize_schema(&self) -> Result<()> {
        // Create accounts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                pubkey TEXT PRIMARY KEY,
                owner TEXT NOT NULL,
                lamports BIGINT NOT NULL,
                slot BIGINT NOT NULL,
                executable BOOLEAN NOT NULL,
                rent_epoch BIGINT NOT NULL,
                data BYTEA,
                write_version BIGINT NOT NULL,
                is_startup BOOLEAN NOT NULL DEFAULT FALSE,
                transaction_signature TEXT,
                last_updated TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS accounts_slot_idx ON accounts(slot);
            CREATE INDEX IF NOT EXISTS accounts_owner_idx ON accounts(owner);
            "#
        )
        .execute(&self.pool)
        .await?;
        
        // Create transactions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transactions (
                signature TEXT PRIMARY KEY,
                slot BIGINT NOT NULL,
                is_vote BOOLEAN NOT NULL,
                message BYTEA,
                meta JSONB,
                index BIGINT NOT NULL,
                last_updated TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS transactions_slot_idx ON transactions(slot);
            "#
        )
        .execute(&self.pool)
        .await?;
        
        // Create blocks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blocks (
                slot BIGINT PRIMARY KEY,
                blockhash TEXT,
                parent_blockhash TEXT,
                last_updated TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            );
            "#
        )
        .execute(&self.pool)
        .await?;
        
        // Create transaction_mentions table for efficient querying
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transaction_mentions (
                signature TEXT NOT NULL REFERENCES transactions(signature) ON DELETE CASCADE,
                pubkey TEXT NOT NULL,
                is_signer BOOLEAN NOT NULL,
                is_writable BOOLEAN NOT NULL,
                PRIMARY KEY (signature, pubkey)
            );
            
            CREATE INDEX IF NOT EXISTS transaction_mentions_pubkey_idx ON transaction_mentions(pubkey);
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn insert_account(&self, account: &AccountData) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO accounts (pubkey, owner, lamports, slot, executable, rent_epoch, data, write_version, is_startup, transaction_signature)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (pubkey) 
            DO UPDATE SET 
                owner = EXCLUDED.owner,
                lamports = EXCLUDED.lamports,
                slot = EXCLUDED.slot,
                executable = EXCLUDED.executable,
                rent_epoch = EXCLUDED.rent_epoch,
                data = EXCLUDED.data,
                write_version = EXCLUDED.write_version,
                is_startup = EXCLUDED.is_startup,
                transaction_signature = EXCLUDED.transaction_signature,
                last_updated = CURRENT_TIMESTAMP
            WHERE accounts.slot <= EXCLUDED.slot OR 
                  (accounts.slot = EXCLUDED.slot AND accounts.write_version < EXCLUDED.write_version)
            "#
        )
        .bind(&account.pubkey)
        .bind(&account.owner)
        .bind(account.lamports as i64)
        .bind(account.slot as i64)
        .bind(account.executable)
        .bind(account.rent_epoch as i64)
        .bind(&account.data.as_slice())
        .bind(account.write_version as i64)
        .bind(account.is_startup)
        .bind(&account.transaction_signature)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn account_from_row(row: PgRow) -> Result<AccountData> {
        let account = AccountData {
            pubkey: row.try_get("pubkey")?,
            owner: row.try_get("owner")?,
            lamports: row.try_get::<i64, _>("lamports")? as u64,
            slot: row.try_get::<i64, _>("slot")? as u64,
            executable: row.try_get("executable")?,
            rent_epoch: row.try_get::<i64, _>("rent_epoch")? as u64,
            data: row.try_get::<Vec<u8>, _>("data")?,
            write_version: row.try_get::<i64, _>("write_version")? as u64,
            is_startup: false,
            transaction_signature: None,
        };
        
        Ok(account)
    }
}

#[async_trait]
impl Storage for PostgresStore {
    async fn store_account(&self, account: AccountData) -> Result<()> {
        self.insert_account(&account).await
    }
    
    async fn store_transaction(&self, transaction: TransactionData) -> Result<()> {
        // Begin transaction
        let mut tx = self.pool.begin().await?;
        
        // Insert transaction
        sqlx::query(
            r#"
            INSERT INTO transactions (signature, slot, is_vote, message, meta, index)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signature) 
            DO UPDATE SET 
                slot = EXCLUDED.slot,
                is_vote = EXCLUDED.is_vote,
                message = EXCLUDED.message,
                meta = EXCLUDED.meta,
                index = EXCLUDED.index,
                last_updated = CURRENT_TIMESTAMP
            "#
        )
        .bind(&transaction.signature)
        .bind(transaction.slot as i64)
        .bind(transaction.is_vote)
        .bind(&transaction.message)
        .bind(&serde_json::to_value(&transaction.meta).unwrap_or_default())
        .bind(transaction.index as i64)
        .execute(&mut tx)
        .await?;
        
        // Insert mentions (simplified for brevity)
        
        // Commit transaction
        tx.commit().await?;
        
        Ok(())
    }
    
    async fn store_block(&self, block: BlockData) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO blocks (slot, blockhash, parent_blockhash)
            VALUES ($1, $2, $3)
            ON CONFLICT (slot) 
            DO UPDATE SET 
                blockhash = EXCLUDED.blockhash,
                parent_blockhash = EXCLUDED.parent_blockhash,
                last_updated = CURRENT_TIMESTAMP
            "#
        )
        .bind(block.slot as i64)
        .bind(block.blockhash)
        .bind(block.parent_blockhash)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn get_account(&self, pubkey: &str) -> Result<Option<AccountData>> {
        let row = sqlx::query(
            "SELECT * FROM accounts WHERE pubkey = $1"
        )
        .bind(pubkey)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => Ok(Some(Self::account_from_row(row).await?)),
            None => Ok(None),
        }
    }
    
    async fn get_transaction(&self, signature: &str) -> Result<Option<TransactionData>> {
        // Simplified implementation
        Ok(None)
    }
    
    async fn get_block(&self, slot: u64) -> Result<Option<BlockData>> {
        // Simplified implementation
        Ok(None)
    }
    
    async fn get_recent_accounts(&self, limit: usize) -> Result<Vec<AccountData>> {
        let rows = sqlx::query(
            "SELECT * FROM accounts ORDER BY last_updated DESC LIMIT $1"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut accounts = Vec::with_capacity(rows.len());
        for row in rows {
            accounts.push(Self::account_from_row(row).await?);
        }
        
        Ok(accounts)
    }
    
    async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<TransactionData>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn get_recent_blocks(&self, limit: usize) -> Result<Vec<BlockData>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn get_accounts_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<AccountData>> {
        let rows = sqlx::query(
            "SELECT * FROM accounts WHERE slot BETWEEN $1 AND $2 ORDER BY slot, write_version LIMIT $3"
        )
        .bind(start_slot as i64)
        .bind(end_slot as i64)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        
        let mut accounts = Vec::with_capacity(rows.len());
        for row in rows {
            accounts.push(Self::account_from_row(row).await?);
        }
        
        Ok(accounts)
    }
    
    async fn get_transactions_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<TransactionData>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn get_blocks_by_slot_range(&self, start_slot: u64, end_slot: u64, limit: usize) -> Result<Vec<BlockData>> {
        // Simplified implementation
        Ok(Vec::new())
    }
    
    async fn close(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
} 