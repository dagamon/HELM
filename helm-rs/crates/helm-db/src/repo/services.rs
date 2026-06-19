use crate::repo::{de_json, ser_json};
use anyhow::Result;
use helm_core::models::{ServiceCreate, ServiceResponse, ServiceUpdate};
use sqlx::{FromRow, Row, SqlitePool};

#[derive(Debug, Clone, FromRow)]
pub struct ServiceRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub r#type: String,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub venv_path: Option<String>,
    pub args: Option<String>,
    pub env: Option<String>,
    pub url: Option<String>,
    pub health_check_url: Option<String>,
    pub health_check_interval: i64,
    pub auto_start: i64,
    pub restart_on_crash: i64,
    pub platform: String,
    pub tags: Option<String>,
    pub depends_on: Option<String>,
    pub webhook_url: Option<String>,
    pub manifest_path: Option<String>,
    pub binary_path: Option<String>,
    pub cargo_profile: Option<String>,
    pub cargo_features: Option<String>,
    pub prebuild: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl ServiceRow {
    pub fn into_response(self, status: String, pid: Option<i64>) -> Result<ServiceResponse> {
        Ok(ServiceResponse {
            id: self.id,
            name: self.name,
            description: self.description,
            r#type: self.r#type,
            command: self.command,
            cwd: self.cwd,
            venv_path: self.venv_path,
            args: de_json(&self.args)?,
            env: de_json(&self.env)?,
            url: self.url,
            health_check_url: self.health_check_url,
            health_check_interval: self.health_check_interval,
            auto_start: self.auto_start != 0,
            restart_on_crash: self.restart_on_crash != 0,
            platform: self.platform,
            tags: de_json(&self.tags)?,
            depends_on: de_json(&self.depends_on)?,
            webhook_url: self.webhook_url,
            manifest_path: self.manifest_path,
            binary_path: self.binary_path,
            cargo_profile: self.cargo_profile,
            cargo_features: self.cargo_features,
            prebuild: self.prebuild != 0,
            created_at: self.created_at,
            updated_at: self.updated_at,
            status,
            pid,
        })
    }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<ServiceRow>> {
    let rows = sqlx::query_as::<_, ServiceRow>("SELECT * FROM services ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<ServiceRow>> {
    let row = sqlx::query_as::<_, ServiceRow>("SELECT * FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn create(pool: &SqlitePool, body: &ServiceCreate) -> Result<i64> {
    let args_json = ser_json(&body.args)?;
    let env_json = ser_json(&body.env)?;
    let tags_json = ser_json(&body.tags)?;
    let depends_json = ser_json(&body.depends_on)?;

    let row = sqlx::query(
        r#"INSERT INTO services
           (name, description, type, command, cwd, venv_path, args, env, url,
            health_check_url, health_check_interval, auto_start, restart_on_crash,
            platform, tags, depends_on, webhook_url,
            manifest_path, binary_path, cargo_profile, cargo_features, prebuild)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING id"#,
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.r#type)
    .bind(&body.command)
    .bind(&body.cwd)
    .bind(&body.venv_path)
    .bind(&args_json)
    .bind(&env_json)
    .bind(&body.url)
    .bind(&body.health_check_url)
    .bind(body.health_check_interval)
    .bind(body.auto_start as i64)
    .bind(body.restart_on_crash as i64)
    .bind(&body.platform)
    .bind(&tags_json)
    .bind(&depends_json)
    .bind(&body.webhook_url)
    .bind(&body.manifest_path)
    .bind(&body.binary_path)
    .bind(&body.cargo_profile)
    .bind(&body.cargo_features)
    .bind(body.prebuild as i64)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>(0))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let res = sqlx::query("DELETE FROM services WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Build dynamic UPDATE SET clause from ServiceUpdate, executing in one statement.
pub async fn update(pool: &SqlitePool, id: i64, body: &ServiceUpdate) -> Result<bool> {
    let mut setters: Vec<&str> = Vec::new();
    let mut binds_str: Vec<Option<String>> = Vec::new();
    let mut binds_i64: Vec<(usize, i64)> = Vec::new();

    macro_rules! push_str {
        ($col:literal, $v:expr) => {
            if let Some(val) = $v {
                setters.push(concat!($col, " = ?"));
                binds_str.push(Some(val.clone()));
            }
        };
    }
    macro_rules! push_str_opt {
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
    macro_rules! push_int {
        ($col:literal, $v:expr) => {
            if let Some(val) = $v {
                setters.push(concat!($col, " = ?"));
                binds_i64.push((setters.len() - 1, val));
            }
        };
    }
    macro_rules! push_bool {
        ($col:literal, $v:expr) => {
            if let Some(val) = $v {
                setters.push(concat!($col, " = ?"));
                binds_i64.push((setters.len() - 1, val as i64));
            }
        };
    }

    push_str!("name", &body.name);
    push_str_opt!("description", &body.description);
    push_str!("type", &body.r#type);
    push_str_opt!("command", &body.command);
    push_str_opt!("cwd", &body.cwd);
    push_str_opt!("venv_path", &body.venv_path);
    push_json!("args", &body.args);
    push_json!("env", &body.env);
    push_str_opt!("url", &body.url);
    push_str_opt!("health_check_url", &body.health_check_url);
    push_int!("health_check_interval", body.health_check_interval);
    push_bool!("auto_start", body.auto_start);
    push_bool!("restart_on_crash", body.restart_on_crash);
    push_str!("platform", &body.platform);
    push_json!("tags", &body.tags);
    push_json!("depends_on", &body.depends_on);
    push_str_opt!("webhook_url", &body.webhook_url);
    push_str_opt!("manifest_path", &body.manifest_path);
    push_str_opt!("binary_path", &body.binary_path);
    push_str_opt!("cargo_profile", &body.cargo_profile);
    push_str_opt!("cargo_features", &body.cargo_features);
    push_bool!("prebuild", body.prebuild);

    if setters.is_empty() {
        return Ok(true);
    }

    setters.push("updated_at = datetime('now')");
    let sql = format!("UPDATE services SET {} WHERE id = ?", setters.join(", "));

    // Build query with binds in order (str positions are stored, int positions by index)
    let mut q = sqlx::query(&sql);
    let int_map: std::collections::HashMap<usize, i64> = binds_i64.into_iter().collect();

    // Walk setters minus the final updated_at to bind params positionally
    let param_count = setters.len() - 1;
    let mut str_idx = 0;
    for i in 0..param_count {
        if let Some(&v) = int_map.get(&i) {
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
