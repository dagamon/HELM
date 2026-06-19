//! Migration tests: empty DB ends in the same schema as fixture, idempotent re-run.

use regex::Regex;
use std::{collections::HashSet, path::PathBuf};
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_schema() -> String {
    let p = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("schema.sql");
    std::fs::read_to_string(p).unwrap()
}

fn normalise(sql: &str) -> HashSet<String> {
    let split = Regex::new(r";\s*[\r\n]+").unwrap();
    let ws = Regex::new(r"\s+").unwrap();
    let trail = Regex::new(r"[;\s]+$").unwrap();
    let mut set = HashSet::new();
    for raw in split.split(sql) {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        let s = trail.replace(s, "").to_string();
        let n = ws.replace_all(&s, " ").trim().to_string();
        if !n.is_empty() {
            set.insert(n);
        }
    }
    set
}

async fn dump_schema(db: &helm_db::Db) -> String {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    rows.into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join(";\n")
}

#[tokio::test]
async fn empty_db_yields_fixture_schema() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("fresh.db");
    let url = format!("sqlite://{}", db_path.to_string_lossy().replace('\\', "/"));

    let db = helm_db::Db::connect(&url).await.expect("connect");
    let dumped = dump_schema(&db).await;

    let fixture = fixture_schema();
    let actual = normalise(&dumped);
    let expected = normalise(&fixture);

    let missing: Vec<_> = expected.difference(&actual).collect();
    let extra: Vec<_> = actual.difference(&expected).collect();

    for m in &missing {
        eprintln!("MISSING: {}", &m[..m.len().min(150)]);
    }
    for e in &extra {
        eprintln!("EXTRA:   {}", &e[..e.len().min(150)]);
    }
    assert!(
        missing.is_empty() && extra.is_empty(),
        "schema mismatch after migrations: {} missing, {} extra",
        missing.len(),
        extra.len()
    );
}

#[tokio::test]
async fn migrations_idempotent() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("idempotent.db");
    let url = format!("sqlite://{}", db_path.to_string_lossy().replace('\\', "/"));

    let db = helm_db::Db::connect(&url).await.expect("first connect");
    let first = dump_schema(&db).await;
    drop(db);

    let db = helm_db::Db::connect(&url).await.expect("second connect");
    let second = dump_schema(&db).await;

    assert_eq!(
        normalise(&first),
        normalise(&second),
        "second migration run changed the schema"
    );
}
