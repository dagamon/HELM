#!/usr/bin/env bash
set -euo pipefail

# HELM Linux Service Installer (Rust backend, systemd)
#
# Generic distro installer. Builds helm (Rust) + static/ (React) in place,
# installs systemd unit pointing at the built binary. Run from anywhere —
# uses repo path the script sits in.
#
# Arch users: prefer install-arch.sh (pacman dep hints).

HELM_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SERVICE_FILE="/etc/systemd/system/helm.service"
RUN_USER="${SUDO_USER:-$(whoami)}"

echo "=== HELM Linux Installer (Rust) ==="
echo "HELM Dir: $HELM_DIR"
echo "Run as user: $RUN_USER"

# --- Toolchain checks ---
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install rustup."; exit 1; }
command -v npm   >/dev/null 2>&1 || { echo "ERROR: npm not found. Install Node.js >= 20."; exit 1; }
NODE_MAJOR=$(node -e "process.stdout.write(process.versions.node.split('.')[0])" 2>/dev/null || echo "0")
if [ "$NODE_MAJOR" -lt 20 ]; then
    echo "ERROR: Node.js $NODE_MAJOR detected; >= 20 required."
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
    (cd "$HELM_DIR/client" && npm install --silent && npm run build --silent)
else
    echo "Frontend already built, skipping."
fi

# --- Install helmctl CLI (optional Python wrapper) ---
if command -v python3 >/dev/null 2>&1; then
    VENV_DIR="$HELM_DIR/.venv"
    [ -d "$VENV_DIR" ] || python3 -m venv "$VENV_DIR"
    "$VENV_DIR/bin/pip" install -e "$HELM_DIR" --quiet
    if [ -f "$VENV_DIR/bin/helmctl" ]; then
        sudo ln -sf "$VENV_DIR/bin/helmctl" /usr/local/bin/helmctl
        echo "helmctl installed → /usr/local/bin/helmctl"
    fi
fi

# --- .env scaffolding ---
mkdir -p "$HELM_DIR/data"
if [ ! -f "$HELM_DIR/.env" ]; then
    cat > "$HELM_DIR/.env" <<'ENV'
HOST=0.0.0.0
PORT=7010
DB_PATH=./data/dashboard.db
DASHBOARD_PIN=
ENV
    echo "Created default $HELM_DIR/.env (no PIN auth)."
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
echo "HELM Rust listening on http://0.0.0.0:7010 (or whatever HOST/PORT in .env)."
echo ""
echo "Management:"
echo "  sudo systemctl status helm"
echo "  sudo systemctl stop helm"
echo "  sudo systemctl restart helm"
echo "  journalctl -u helm -f"
