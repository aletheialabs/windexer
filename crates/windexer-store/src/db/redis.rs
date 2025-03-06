// crates/windexer-store/src/db/redis.rs

use crate::db::Database;
use crate::schema::Schema;
use crate::store::Store;

pub struct RedisDatabase {
    client: RedisClient,
    schema: Schema,
}

impl RedisDatabase {
    pub fn new(client: RedisClient) -> Self {
        let schema = Schema::new(&client);
        Self { client, schema }
    }
}

impl Database for RedisDatabase {
    fn get_schema(&self) -> &Schema {
        &self.schema
    }
}

impl Store for RedisDatabase {
    fn get_store(&self) -> &Store {
        &self.store
    }
}

impl Drop for RedisDatabase {
    fn drop(&mut self) {
        self.client.close();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;
    use crate::store::Store;
    use redis::Client;

    #[tokio::test]
    async fn test_redis_database() {
        let client = Client::open("redis://localhost:6379").unwrap();
        let db = RedisDatabase::new(client);
    }
}
