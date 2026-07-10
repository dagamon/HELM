use crate::repo::services::ServiceRow;
use crate::repo::{de_json, ser_json};
use anyhow::Result;
use helm_core::models::{StackCreate, StackResponse, StackUpdate};
use sqlx::{FromRow, Row, SqlitePool};

#[derive(Debug, Clone, FromRow)]
pub struct StackRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<String>,
    pub card_color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl StackRow {
    pub fn into_response(self, service_count: i64, running_count: i64) -> Result<StackResponse> {
        let status = if service_count == 0 || running_count == 0 {
            "stopped"
        } else if running_count == service_count {
            "running"
        } else {
            "partial"
        };
        Ok(StackResponse {
            id: self.id,
            name: self.name,
            description: self.description,
            tags: de_json(&self.tags)?,
            card_color: self.card_color,
            created_at: self.created_at,
            updated_at: self.updated_at,
            service_count,
            running_count,
            status: status.into(),
        })
    }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<StackRow>> {
    let rows = sqlx::query_as::<_, StackRow>("SELECT * FROM stacks ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<StackRow>> {
    let row = sqlx::query_as::<_, StackRow>("SELECT * FROM stacks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn create(pool: &SqlitePool, body: &StackCreate) -> Result<i64> {
    let tags_json = ser_json(&body.tags)?;
    let row = sqlx::query(
        "INSERT INTO stacks (name, description, tags, card_color) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(&tags_json)
    .bind(&body.card_color)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>(0))
}

pub async fn update(pool: &SqlitePool, id: i64, body: &StackUpdate) -> Result<bool> {
    let mut setters: Vec<&str> = Vec::new();
    let mut binds: Vec<Option<String>> = Vec::new();

    if let Some(name) = &body.name {
        setters.push("name = ?");
        binds.push(Some(name.clone()));
    }
    if let Some(desc) = &body.description {
        setters.push("description = ?");
        binds.push(Some(desc.clone()));
    }
    if body.tags.is_some() {
        setters.push("tags = ?");
        binds.push(ser_json(&body.tags)?);
    }
    if let Some(color) = &body.card_color {
        setters.push("card_color = ?");
        binds.push(Some(color.clone()));
    }
    if setters.is_empty() {
        return Ok(true);
    }
    setters.push("updated_at = datetime('now')");

    let sql = format!("UPDATE stacks SET {} WHERE id = ?", setters.join(", "));
    let mut q = sqlx::query(&sql);
    for b in binds {
        q = q.bind(b);
    }
    let res = q.bind(id).execute(pool).await?;
    Ok(res.rows_affected() > 0)
}

/// Delete a stack; member services survive, detached (stack_id → NULL).
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE services SET stack_id = NULL WHERE stack_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let res = sqlx::query("DELETE FROM stacks WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(res.rows_affected() > 0)
}

pub async fn services_in_stack(pool: &SqlitePool, stack_id: i64) -> Result<Vec<ServiceRow>> {
    let rows = sqlx::query_as::<_, ServiceRow>(
        "SELECT * FROM services WHERE stack_id = ? ORDER BY sort_order, id",
    )
    .bind(stack_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
