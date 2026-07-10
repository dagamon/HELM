CREATE TABLE services (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    name                  TEXT NOT NULL,
    description           TEXT,
    type                  TEXT NOT NULL,
    command               TEXT,
    cwd                   TEXT,
    args                  TEXT,
    env                   TEXT,
    url                   TEXT,
    health_check_url      TEXT,
    health_check_interval INTEGER DEFAULT 30,
    auto_start            INTEGER DEFAULT 0,
    restart_on_crash      INTEGER DEFAULT 0,
    platform              TEXT DEFAULT 'all',
    tags                  TEXT,
    created_at            TEXT DEFAULT (datetime('now')),
    updated_at            TEXT DEFAULT (datetime('now'))
, venv_path TEXT, depends_on TEXT, webhook_url TEXT, manifest_path TEXT, binary_path TEXT, cargo_profile TEXT DEFAULT 'release', cargo_features TEXT, prebuild INTEGER DEFAULT 0, sort_order INTEGER DEFAULT 0, stack_id INTEGER, card_color TEXT);
CREATE TABLE stacks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    tags        TEXT,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now'))
, card_color TEXT);
CREATE TABLE sqlite_sequence(name,seq);
CREATE TABLE scripts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    command     TEXT NOT NULL,
    cwd         TEXT,
    args        TEXT,
    platform    TEXT DEFAULT 'all',
    tags        TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
, cron_schedule TEXT, cron_enabled INTEGER DEFAULT 0, run_mode TEXT DEFAULT 'exec');
CREATE TABLE run_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id   INTEGER NOT NULL,
    started_at  TEXT DEFAULT (datetime('now')),
    stopped_at  TEXT,
    exit_code   INTEGER,
    status      TEXT,
    pid         INTEGER
);
CREATE TABLE output_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id   INTEGER NOT NULL,
    stream      TEXT NOT NULL,
    line        TEXT NOT NULL,
    ts          TEXT DEFAULT (datetime('now'))
);
CREATE INDEX idx_output_logs
    ON output_logs(entity_type, entity_id, ts DESC);
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
