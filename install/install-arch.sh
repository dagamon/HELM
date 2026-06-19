#!/usr/bin/env bash
set -euo pipefail

# HELM Arch Linux Installer (Rust backend, systemd user/system unit)
#
# Builds helm (Rust) + static/ (React) in place, installs systemd unit.
# Run from anywhere — uses repo path the script sits in.
# Assumes pacman-based system (Arch / Manjaro / EndeavourOS).

HELM_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SERVICE_FILE="/etc/systemd/system/helm.service"
RUN_USER="${SUDO_USER:-$(whoami)}"

echo "=== HELM Arch Installer (Rust) ==="
echo "HELM Dir: $HELM_DIR"
echo "Run as user: $RUN_USER"

# --- Dependency check (pacman) ---
missing=()
command -v cargo >/dev/null 2>&1 || missing+=("rust")
command -v npm   >/dev/null 2>&1 || missing+=("nodejs npm")
command -v sqlite3 >/dev/null 2>&1 || missing+=("sqlite")

if [ ${#missing[@]} -gt 0 ]; then
    echo "Missing packages: ${missing[*]}"
    echo "Install via: sudo pacman -S --needed ${missing[*]}"
    exit 1
fi

NODE_MAJOR=$(node -e "process.stdout.write(process.versions.node.split('.')[0])" 2>/dev/null || echo "0")
if [ "$NODE_MAJOR" -lt 20 ]; then
    echo "ERROR: Node.js $NODE_MAJOR detected; >= 20 required (pacman -S nodejs)."
    exit 1
fi

# --- Stop existing service so cargo can overwrite the running binary ---
if systemctl is-active --quiet helm 2>/dev/null; then
    echo "Stopping existing helm.service..."
    sudo systemctl stop helm
fi

# --- Build Rust backend ---
echo "Building helm (cargo build --release)..."
cargo build --release --manifest-path "$HELM_DIR/helm-rs/Cargo.toml" -p helm-bin

BIN_SRC="$HELM_DIR/helm-rs/target/release/helm"
[ -x "$BIN_SRC" ] || { echo "ERROR: built binary missing at $BIN_SRC"; exit 1; }
cp "$BIN_SRC" "$HELM_DIR/helm"
echo "Binary placed at $HELM_DIR/helm"

# --- Build frontend ---
if [ ! -f "$HELM_DIR/static/index.html" ]; then
    echo "Building frontend (Node.js $NODE_MAJOR)..."
    (cd "$HELM_DIR/client" && npm install --silent && npm run build)
else
    echo "Frontend already built, skipping."
fi

# --- helmctl CLI (optional) ---
if command -v python3 >/dev/null 2>&1; then
    VENV_DIR="$HELM_DIR/.venv"
    [ -d "$VENV_DIR" ] || python3 -m venv "$VENV_DIR"
    "$VENV_DIR/bin/pip" install -e "$HELM_DIR" --quiet
    if [ -f "$VENV_DIR/bin/helmctl" ]; then
        sudo ln -sf "$VENV_DIR/bin/helmctl" /usr/local/bin/helmctl
        echo "helmctl → /usr/local/bin/helmctl"
    fi
fi

# --- .env scaffolding ---
mkdir -p "$HELM_DIR/data"
if [ ! -f "$HELM_DIR/.env" ]; then
    cat > "$HELM_DIR/.env" <<'ENV'
HOST=127.0.0.1
PORT=7010
DB_PATH=./data/dashboard.db
DASHBOARD_PIN=
ENV
    echo "Created default $HELM_DIR/.env"
fi

# --- systemd unit ---
echo "Installing systemd service..."
sudo tee "$SERVICE_FILE" >/dev/null <<EOF
[Unit]
Description=HELM Service Control Dashboard (Rust)
After=network.target

[Service]
Type=simple
User=$RUN_USER
WorkingDirectory=$HELM_DIR
ExecStart=$HELM_DIR/helm
Restart=on-failure
RestartSec=5
EnvironmentFile=$HELM_DIR/.env
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable helm
sudo systemctl restart helm

echo ""
echo "=== Done! ==="
echo "HELM Rust listening per HOST/PORT in .env (default http://127.0.0.1:7010)."
echo ""
echo "Management:"
echo "  sudo systemctl status helm"
echo "  sudo systemctl stop helm"
echo "  sudo systemctl restart helm"
echo "  journalctl -u helm -f"
