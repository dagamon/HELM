//! Idempotent schema migrator.
//!
//! Phase 1: apply `0001_init.sql` (uses `IF NOT EXISTS` — safe on existing DBs).
//! Phase 2: for each historical ALTER, check column existence via `pragma_table_info`
//! and apply the ALTER only if the column is missing. Mirrors the behaviour of the
//! Python `Database._migrate()` so that an upgraded prod DB ends up with byte-identical
//! `sqlite_master.sql` strings.

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tracing::info;

const INIT_SQL: &str = include_str!("../migrations/0001_init.sql");

const COLUMN_MIGRATIONS: &[(&str, &str, &str)] = &[
    (
        "services",
        "venv_path",
        include_str!("../migrations/0002_service_venv_path.sql"),
    ),
    (
        "services",
        "depends_on",
        include_str!("../migrations/0003_service_depends_on.sql"),
    ),
    (
        "services",
        "webhook_url",
        include_str!("../migrations/0004_service_webhook_url.sql"),
    ),
    (
        "scripts",
        "cron_schedule",
        include_str!("../migrations/0005_script_cron_schedule.sql"),
    ),
    (
        "scripts",
        "cron_enabled",
        include_str!("../migrations/0006_script_cron_enabled.sql"),
    ),
    (
        "scripts",
        "run_mode",
        include_str!("../migrations/0007_script_run_mode.sql"),
    ),
    (
        "services",
        "manifest_path",
        include_str!("../migrations/0008_service_manifest_path.sql"),
    ),
    (
        "services",
        "binary_path",
        include_str!("../migrations/0009_service_binary_path.sql"),
    ),
    (
        "services",
        "cargo_profile",
        include_str!("../migrations/0010_service_cargo_profile.sql"),
    ),
    (
        "services",
        "cargo_features",
        include_str!("../migrations/0011_service_cargo_features.sql"),
    ),
    (
        "services",
        "prebuild",
        include_str!("../migrations/0012_service_prebuild.sql"),
    ),
];

pub async fn run(pool: &SqlitePool) -> Result<()> {
    apply_script(pool, INIT_SQL).await.context("0001_init")?;

    for (table, column, sql) in COLUMN_MIGRATIONS {
        if !column_exists(pool, table, column).await? {
            apply_script(pool, sql)
                .await
                .with_context(|| format!("add column {table}.{column}"))?;
            info!("Migration: added {}.{}", table, column);
        }
    }
    Ok(())
}

async fn apply_script(pool: &SqlitePool, sql: &str) -> Result<()> {
    for stmt in split_statements(sql) {
        sqlx::query(&stmt).execute(pool).await?;
    }
    Ok(())
}

fn split_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("{};", s))
        .collect()
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
    // Table name is hardcoded compile-time, safe to interpolate.
    let q = format!(
        "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?",
        table
    );
    let (count,): (i64,) = sqlx::query_as(&q).bind(column).fetch_one(pool).await?;
    Ok(count > 0)
}
