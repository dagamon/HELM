# HELM

Service control dashboard for running local tools, bots, dev servers, and maintenance scripts from one self-hosted web UI.

HELM runs at `http://127.0.0.1:7010` by default and is designed to live as a background service under NSSM on Windows or systemd on Linux. The backend is Rust; the frontend is React/Vite; the database is SQLite.

## Features

- Start, stop, and restart long-running services
- Run one-shot scripts and inspect run history
- Stream stdout/stderr logs in real time over WebSocket
- Persist logs to SQLite with automatic retention cleanup
- Schedule scripts with cron expressions
- Monitor services with HTTP health checks and process metrics
- Model service dependencies and restart-on-crash behavior
- Manage Rust services with Cargo manifest/profile/features support
- Export and import service/script configuration as JSON
- Optional PIN authentication for REST and WebSocket endpoints
- Python `helmctl` CLI wrapper for automation and terminal workflows

## Stack

| Area | Technology |
|---|---|
| Backend | Rust, Tokio, Axum, tower-http |
| Database | SQLite via SQLx, WAL mode, idempotent migrations |
| Process control | `tokio::process::Command`, Windows JobObject, Unix process groups |
| Metrics | `sysinfo` |
| Scheduling | `tokio-cron-scheduler` |
| Frontend | React 19, Vite 7, Tailwind CSS 4, Zustand, xterm.js |
| CLI | Python 3.11+, Typer, httpx, websockets |
| Service manager | NSSM on Windows, systemd on Linux |

## Repository Layout

```text
helm-rs/                    Rust backend workspace
  crates/helm-bin/          Binary entry point and runtime setup
  crates/helm-api/          Axum REST/WebSocket API and PIN middleware
  crates/helm-core/         Shared models and errors
  crates/helm-db/           SQLite connection, migrations, repositories
  crates/helm-platform/     Platform command resolution and env handling
  crates/helm-proc/         Process manager, logs, metrics, scheduler
client/                     React frontend
cli/                        Python helmctl CLI
static/                     Built frontend served by the Rust binary
data/                       Runtime database and service logs
install/                    NSSM/systemd installers
tests/fixtures/             API and runtime fixtures
```

## Requirements

- Rust toolchain with Cargo
- Node.js 20+ and npm
- SQLite runtime
- NSSM on Windows, or systemd on Linux
- Python 3.11+ only if you want to install `helmctl`

## Quick Start

```bash
git clone https://github.com/dagamon/HELM.git
cd HELM
cp .env.example .env
```

Build the backend:

```bash
cargo build --release --manifest-path helm-rs/Cargo.toml -p helm-bin
```

Build the frontend:

```bash
cd client
npm install
npm run build
cd ..
```

Run directly for local debugging:

```bash
# Linux/macOS
./helm-rs/target/release/helm

# Windows
.\helm-rs\target\release\helm.exe
```

Open `http://127.0.0.1:7010`.

## Install as a Service

### Windows

Run from an Administrator terminal with `nssm.exe`, Cargo, Node.js, and npm available on `PATH`:

```bat
git clone https://github.com/dagamon/HELM.git
cd HELM
install\install-windows.bat
```

The installer builds `helm-rs\target\release\helm.exe`, ensures `static/` and `data/` exist, creates `.env` if missing, installs the `HELM` NSSM service, and starts it.

Management:

```bat
nssm status HELM
nssm stop HELM
nssm start HELM
nssm restart HELM
nssm remove HELM confirm
```

### Linux

Generic systemd installer:

```bash
git clone https://github.com/dagamon/HELM.git
cd HELM
bash install/install-linux.sh
```

Arch/Manjaro/EndeavourOS users can use the pacman-aware installer:

```bash
bash install/install-arch.sh
```

The Linux installers build the Rust binary, copy it to `./helm`, build the frontend when needed, create a systemd unit at `/etc/systemd/system/helm.service`, and restart the service.

Management:

```bash
sudo systemctl status helm
sudo systemctl stop helm
sudo systemctl restart helm
journalctl -u helm -f
```

## Updating

Pull changes first:

```bash
git pull --ff-only
```

Windows:

```bat
install\install-windows.bat
```

Linux:

```bash
cargo build --release --manifest-path helm-rs/Cargo.toml -p helm-bin
cd client && npm install && npm run build && cd ..
cp helm-rs/target/release/helm ./helm
sudo systemctl restart helm
```

The Windows installer stops the existing NSSM service before rebuilding so Cargo can overwrite `helm.exe`. On Linux, stop the service first if your current setup runs the binary directly from `helm-rs/target/release/helm`.

The frontend build is written to `static/`, which is served directly by the Rust binary.

## Configuration

Copy `.env.example` to `.env` and edit as needed:

```env
HOST=127.0.0.1
PORT=7010
DB_PATH=./data/dashboard.db
LOG_BUFFER_SIZE=500
LOG_LINES_KEEP=1000
OUTPUT_LOGS_RETENTION_DAYS=14
OUTPUT_LOGS_MAX_ROWS_TOTAL=300000
OUTPUT_LOGS_MAX_ROWS_PER_ENTITY=100000
OUTPUT_LOGS_CLEANUP_INTERVAL_SECONDS=300
OUTPUT_LOGS_VACUUM_INTERVAL_SECONDS=21600
DASHBOARD_PIN=
```

If `DASHBOARD_PIN` is empty, authentication is disabled. If it is set, clients must login via `/api/login`, send the `helm_pin` cookie, or provide the `X-PIN` header.

## API Overview

```text
GET    /api/health
GET    /api/system/info
GET    /api/export
POST   /api/import
POST   /api/restart-server
POST   /api/login

GET    /api/services
POST   /api/services
GET    /api/services/{id}
PUT    /api/services/{id}
DELETE /api/services/{id}
POST   /api/services/{id}/start
POST   /api/services/{id}/stop
POST   /api/services/{id}/restart
GET    /api/services/{id}/logs
GET    /api/services/{id}/metrics

GET    /api/scripts
POST   /api/scripts
GET    /api/scripts/{id}
PUT    /api/scripts/{id}
DELETE /api/scripts/{id}
POST   /api/scripts/{id}/run
GET    /api/scripts/runs/{id}
GET    /api/scripts/scheduler/next-run

GET    /api/faq
GET    /api/faq/{slug}

WS     /ws/logs/{entity_type}/{entity_id}
WS     /ws/status
```

## CLI

`helmctl` is a Python CLI client for a running HELM instance. It talks only to the HTTP/WebSocket API and does not access SQLite directly.

Install from the repository root:

```bash
pip install -e .
```

Common commands:

```bash
helmctl health
helmctl info
helmctl ls
helmctl status <service>
helmctl start <service>
helmctl stop <service>
helmctl restart <service>
helmctl logs <service> -f

helmctl script ls
helmctl script run <script> -f
helmctl script status <run-id>

helmctl export backup.json
helmctl import backup.json
```

Environment:

```text
HELM_URL=http://localhost:7010
HELM_PIN=
```

## Development

Backend:

```bash
cargo check --manifest-path helm-rs/Cargo.toml
cargo test --manifest-path helm-rs/Cargo.toml
cargo run --manifest-path helm-rs/Cargo.toml -p helm-bin
```

Frontend:

```bash
cd client
npm install
npm run dev
npm run build
```

The Vite dev server proxies `/api` and `/ws` to `http://127.0.0.1:7010`.

## Migration Notes

HELM used to have a Python FastAPI backend. The backend was rewritten to Rust in May 2026.

Old Python backend paths such as `server/main.py`, `server/requirements.txt`, and uvicorn service commands are obsolete. The live service should point at the Rust binary:

```text
Windows NSSM: helm-rs\target\release\helm.exe
Linux systemd: ./helm
```

SQLite data remains in `data/dashboard.db`; Rust migrations run automatically at startup.
