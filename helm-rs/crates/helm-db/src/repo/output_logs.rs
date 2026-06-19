use anyhow::Result;
use helm_core::models::OutputLogEntry;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow)]
pub struct OutputLogRow {
    pub stream: String,
    pub line: String,
    pub ts: String,
}

impl From<OutputLogRow> for OutputLogEntry {
    fn from(r: OutputLogRow) -> Self {
        OutputLogEntry {
            stream: r.stream,
            line: r.line,
            ts: r.ts,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CleanupStats {
    pub deleted_by_age: i64,
    pub deleted_by_total_cap: i64,
    pub deleted_by_entity_cap: i64,
    pub deleted_total: i64,
    pub remaining_rows: i64,
}

pub async fn cleanup(
    pool: &SqlitePool,
    retention_days: i64,
    max_rows_total: i64,
    max_rows_per_entity: i64,
) -> Result<CleanupStats> {
    let mut stats = CleanupStats::default();

    if retention_days > 0 {
        let cutoff = format!("-{retention_days} days");
        let res = sqlx::query("DELETE FROM output_logs WHERE ts < datetime('now', ?)")
            .bind(&cutoff)
            .execute(pool)
            .await?;
        stats.deleted_by_age = res.rows_affected() as i64;
        stats.deleted_total += stats.deleted_by_age;
    }

    if max_rows_total > 0 {
        let res = sqlx::query(
            "DELETE FROM output_logs WHERE id IN (
                SELECT id FROM output_logs
                ORDER BY ts DESC, id DESC
                LIMIT -1 OFFSET ?
            )",
        )
        .bind(max_rows_total)
        .execute(pool)
        .await?;
        stats.deleted_by_total_cap = res.rows_affected() as i64;
        stats.deleted_total += stats.deleted_by_total_cap;
    }

    if max_rows_per_entity > 0 {
        #[derive(FromRow)]
        struct EntityKey {
            entity_type: String,
            entity_id: i64,
        }
        let entities: Vec<EntityKey> =
            sqlx::query_as("SELECT DISTINCT entity_type, entity_id FROM output_logs")
                .fetch_all(pool)
                .await?;
        let mut deleted = 0i64;
        for e in entities {
            let res = sqlx::query(
                "DELETE FROM output_logs
                 WHERE entity_type = ? AND entity_id = ?
                   AND id IN (
                       SELECT id FROM output_logs
                       WHERE entity_type = ? AND entity_id = ?
                       ORDER BY ts DESC, id DESC
                       LIMIT -1 OFFSET ?
                   )",
            )
            .bind(&e.entity_type)
            .bind(e.entity_id)
            .bind(&e.entity_type)
            .bind(e.entity_id)
            .bind(max_rows_per_entity)
            .execute(pool)
            .await?;
            deleted += res.rows_affected() as i64;
        }
        stats.deleted_by_entity_cap = deleted;
        stats.deleted_total += deleted;
    }

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM output_logs")
        .fetch_one(pool)
        .await?;
    stats.remaining_rows = count.0;
    Ok(stats)
}

pub async fn vacuum(pool: &SqlitePool) -> Result<()> {
    sqlx::query("VACUUM").execute(pool).await?;
    Ok(())
}

pub async fn insert(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: i64,
    stream: &str,
    line: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO output_logs (entity_type, entity_id, stream, line) VALUES (?, ?, ?, ?)",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(stream)
    .bind(line)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch the most recent `limit` lines for an entity, returned in chronological order.
pub async fn recent(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: i64,
    limit: i64,
) -> Result<Vec<OutputLogRow>> {
    let mut rows = sqlx::query_as::<_, OutputLogRow>(
        "SELECT stream, line, ts FROM output_logs
         WHERE entity_type = ? AND entity_id = ?
         ORDER BY ts DESC LIMIT ?",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.reverse();
    Ok(rows)
}
