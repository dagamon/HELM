use crate::{error::ApiError, state::AppState};
use axum::{extract::Path, routing::get, Json, Router};
use helm_core::models::{FaqArticle, FaqArticleContent};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::PathBuf;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/faq/articles", get(list))
        .route("/api/faq/articles/:slug", get(get_article))
}

fn faq_dir() -> PathBuf {
    // docs/faq/ relative to workspace root (helm-rs/../docs/faq)
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
        .join("faq")
}

fn extract_title(content: &str) -> String {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    "Untitled".into()
}

static SLUG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

async fn list() -> Result<Json<Vec<FaqArticle>>, ApiError> {
    let dir = faq_dir();
    if !dir.exists() {
        return Ok(Json(Vec::new()));
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| ApiError::Internal(format!("read faq dir: {e}")))?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry.path();
        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        out.push(FaqArticle {
            slug,
            title: extract_title(&content),
        });
    }
    Ok(Json(out))
}

async fn get_article(Path(slug): Path<String>) -> Result<Json<FaqArticleContent>, ApiError> {
    if !SLUG_RE.is_match(&slug) {
        return Err(ApiError::BadRequest("Invalid slug".into()));
    }
    let path = faq_dir().join(format!("{slug}.md"));
    if !path.exists() {
        return Err(ApiError::NotFound("Article not found".into()));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(format!("read article: {e}")))?;
    Ok(Json(FaqArticleContent {
        slug,
        title: extract_title(&content),
        content,
    }))
}
