// crates/windexer-store/src/db/mod.rs

pub mod db;
pub mod schema;
pub mod store;

pub use db::*;
pub use schema::*;
pub use store::*;

pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_connection() {
        let db = Database::new("test.db").await.unwrap();
        assert!(db.is_ok());
    }

    #[tokio::test]
    async fn test_schema_creation() {
        let db = Database::new("test.db").await.unwrap();
        let schema = Schema::new(&db).await.unwrap();
        assert!(schema.is_ok());
    }
}




            