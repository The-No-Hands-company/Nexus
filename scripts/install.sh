#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Nexus — production install script
#
# Builds release binaries, installs the desktop app, and wires up the
# backend server as a systemd user service (auto-starts on login, no terminal).
#
# Usage: ./scripts/install.sh
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="$HOME/.local/bin"
SERVICE_DIR="$HOME/.config/systemd/user"
DESKTOP_DIR="$HOME/.local/share/applications"
ICONS_DIR="$HOME/.local/share/icons/hicolor"

RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'
info()    { echo -e "${CYAN}[nexus]${RESET} $*"; }
success() { echo -e "${GREEN}[nexus]${RESET} $*"; }
die()     { echo -e "${RED}[nexus] ERROR:${RESET} $*" >&2; exit 1; }
step()    { echo -e "\n${BOLD}── $* ──${RESET}"; }

mkdir -p "$INSTALL_DIR" "$SERVICE_DIR" "$DESKTOP_DIR"

# ── Step 1: Build release server binary ───────────────────────────────────────
step "Building nexus-server (release)"
cd "$ROOT_DIR"
cargo build -p nexus-server --release
cp "$ROOT_DIR/target/release/nexus" "$INSTALL_DIR/nexus-server"
chmod +x "$INSTALL_DIR/nexus-server"
success "nexus-server → $INSTALL_DIR/nexus-server"

# ── Step 2: Build Tauri desktop app ───────────────────────────────────────────
step "Building Nexus desktop app"
cd "$ROOT_DIR/crates/nexus-desktop"
npm install --silent
npm run tauri build

# Find the produced AppImage
APPIMAGE=$(find "$ROOT_DIR/target/release/bundle/appimage" -name "*.AppImage" 2>/dev/null | head -1 || true)
DEB_PKG=$(find "$ROOT_DIR/target/release/bundle/deb"      -name "*.deb"      2>/dev/null | head -1 || true)

if [[ -n "$APPIMAGE" ]]; then
  cp "$APPIMAGE" "$INSTALL_DIR/nexus-desktop"
  chmod +x "$INSTALL_DIR/nexus-desktop"
  success "AppImage → $INSTALL_DIR/nexus-desktop"
elif [[ -n "$DEB_PKG" ]]; then
  info "Installing .deb package (requires sudo)..."
  sudo dpkg -i "$DEB_PKG"
  success ".deb installed"
else
  die "Could not find AppImage or .deb in target/release/bundle — check tauri build output above"
fi

# ── Step 3: Install icon ───────────────────────────────────────────────────────
ICON_SRC="$ROOT_DIR/crates/nexus-desktop/src-tauri/icons/128x128.png"
if [[ -f "$ICON_SRC" ]]; then
  mkdir -p "$ICONS_DIR/128x128/apps"
  cp "$ICON_SRC" "$ICONS_DIR/128x128/apps/nexus.png"
  gtk-update-icon-cache "$ICONS_DIR" 2>/dev/null || true
fi

# ── Step 4: Create .desktop launcher ─────────────────────────────────────────
step "Creating application launcher"
cat > "$DESKTOP_DIR/nexus.desktop" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Nexus
GenericName=Community Chat
Comment=Privacy-first communication — no ID required
Exec=$INSTALL_DIR/nexus-desktop
Icon=nexus
Terminal=false
Categories=Network;InstantMessaging;Chat;
Keywords=chat;messaging;voice;community;discord;
StartupNotify=true
StartupWMClass=nexus
EOF
chmod +x "$DESKTOP_DIR/nexus.desktop"
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
success "Launcher → $DESKTOP_DIR/nexus.desktop"

# ── Step 5: Install systemd user service (backend auto-start) ─────────────────
step "Installing nexus-server systemd user service"
cat > "$SERVICE_DIR/nexus-server.service" <<EOF
[Unit]
Description=Nexus API & Gateway Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=$ROOT_DIR
EnvironmentFile=$ROOT_DIR/.env
ExecStartPre=/bin/sh -c 'docker compose up -d 2>/dev/null || podman-compose up -d'
ExecStart=$INSTALL_DIR/nexus-server
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nexus-server

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable nexus-server.service
systemctl --user start  nexus-server.service
success "nexus-server service enabled and started"

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}${GREEN}Nexus installed!${RESET}"
echo ""
echo "  Desktop app:  search 'Nexus' in your app launcher"
echo "  Server logs:  journalctl --user -u nexus-server -f"
echo "  Stop server:  systemctl --user stop nexus-server"
echo "  Restart:      systemctl --user restart nexus-server"
echo ""
echo "  The backend (nexus-server) and all containers start automatically on"
echo "  login. No terminal needed — just open Nexus from your app launcher."
echo ""
