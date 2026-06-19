use crate::repo::{de_json, ser_json};
use anyhow::Result;
use helm_core::models::{ScriptCreate, ScriptResponse, ScriptUpdate};
use sqlx::{FromRow, Row, SqlitePool};

#[derive(Debug, Clone, FromRow)]
pub struct ScriptRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub run_mode: Option<String>,
    pub cwd: Option<String>,
    pub args: Option<String>,
    pub platform: String,
    pub tags: Option<String>,
    pub cron_schedule: Option<String>,
    pub cron_enabled: i64,
    pub created_at: String,
}

impl ScriptRow {
    pub fn into_response(self) -> Result<ScriptResponse> {
        Ok(ScriptResponse {
            id: self.id,
            name: self.name,
            description: self.description,
            command: self.command,
            run_mode: self.run_mode.unwrap_or_else(|| "exec".into()),
            cwd: self.cwd,
            args: de_json(&self.args)?,
            platform: self.platform,
            tags: de_json(&self.tags)?,
            cron_schedule: self.cron_schedule,
            cron_enabled: self.cron_enabled != 0,
            created_at: self.created_at,
        })
    }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<ScriptRow>> {
    let rows = sqlx::query_as::<_, ScriptRow>("SELECT * FROM scripts ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<ScriptRow>> {
    let row = sqlx::query_as::<_, ScriptRow>("SELECT * FROM scripts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn create(pool: &SqlitePool, body: &ScriptCreate) -> Result<i64> {
    let args_json = ser_json(&body.args)?;
    let tags_json = ser_json(&body.tags)?;

    let row = sqlx::query(
        r#"INSERT INTO scripts
           (name, description, command, run_mode, cwd, args, platform, tags,
            cron_schedule, cron_enabled)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING id"#,
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.command)
    .bind(&body.run_mode)
    .bind(&body.cwd)
    .bind(&args_json)
    .bind(&body.platform)
    .bind(&tags_json)
    .bind(&body.cron_schedule)
    .bind(body.cron_enabled as i64)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>(0))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let res = sqlx::query("DELETE FROM scripts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn update(pool: &SqlitePool, id: i64, body: &ScriptUpdate) -> Result<bool> {
    let mut setters: Vec<&str> = Vec::new();
    let mut binds_str: Vec<Option<String>> = Vec::new();
    let mut binds_i64: std::collections::HashMap<usize, i64> = std::collections::HashMap::new();

    macro_rules! push_str {
        ($col:literal, $v:expr) => {
            if let Some(val) = $v {
                setters.push(concat!($col, " = ?"));
                binds_str.push(Some(val.clone()));
            }
        };
    }
    macro_rules! push_json {
        ($col:literal, $v:expr) => {
            if $v.is_some() {
                let s = ser_json($v)?;
                setters.push(concat!($col, " = ?"));
                binds_str.push(s);
            }
        };
    }
    macro_rules! push_bool {
        ($col:literal, $v:expr) => {
            if let Some(val) = $v {
                setters.push(concat!($col, " = ?"));
                binds_i64.insert(setters.len() - 1, val as i64);
            }
        };
    }

    push_str!("name", &body.name);
    push_str!("description", &body.description);
    push_str!("command", &body.command);
    push_str!("run_mode", &body.run_mode);
    push_str!("cwd", &body.cwd);
    push_json!("args", &body.args);
    push_str!("platform", &body.platform);
    push_json!("tags", &body.tags);
    push_str!("cron_schedule", &body.cron_schedule);
    push_bool!("cron_enabled", body.cron_enabled);

    if setters.is_empty() {
        return Ok(true);
    }

    let sql = format!("UPDATE scripts SET {} WHERE id = ?", setters.join(", "));
    let mut q = sqlx::query(&sql);
    let mut str_idx = 0;
    for i in 0..setters.len() {
        if let Some(&v) = binds_i64.get(&i) {
            q = q.bind(v);
        } else {
            q = q.bind(binds_str[str_idx].clone());
            str_idx += 1;
        }
    }
    q = q.bind(id);

    let res = q.execute(pool).await?;
    Ok(res.rows_affected() > 0)
}
