use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use tracing::info;

pub mod migrations;
pub mod repo;

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn connect(db_path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(db_path)?
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5))
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(opts).await?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        migrations::run(&pool).await?;

        info!("Database connected: {}", db_path);
        Ok(Self { pool })
    }

    pub async fn service_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM services")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    pub async fn script_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scripts")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}
