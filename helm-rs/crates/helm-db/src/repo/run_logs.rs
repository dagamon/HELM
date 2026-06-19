use anyhow::Result;
use helm_core::models::RunLog;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow)]
pub struct RunLogRow {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub exit_code: Option<i64>,
    pub status: Option<String>,
    pub pid: Option<i64>,
}

impl From<RunLogRow> for RunLog {
    fn from(r: RunLogRow) -> Self {
        RunLog {
            id: r.id,
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            started_at: r.started_at,
            stopped_at: r.stopped_at,
            exit_code: r.exit_code,
            status: r.status,
            pid: r.pid,
        }
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<RunLogRow>> {
    let row = sqlx::query_as::<_, RunLogRow>("SELECT * FROM run_logs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn insert(pool: &SqlitePool, entity_type: &str, entity_id: i64, pid: u32) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO run_logs (entity_type, entity_id, status, pid) VALUES (?, ?, 'running', ?)",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(pid as i64)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_stopped(
    pool: &SqlitePool,
    id: i64,
    exit_code: Option<i32>,
    status: &str,
) -> Result<()> {
    sqlx::query("UPDATE run_logs SET stopped_at=datetime('now'), exit_code=?, status=? WHERE id=?")
        .bind(exit_code.map(|c| c as i64))
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
