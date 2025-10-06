use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub mod models;
pub mod queries;

pub type DbPool = PgPool;

pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./sql").run(pool).await
}
