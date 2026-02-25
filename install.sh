#!/usr/bin/env sh
# install.sh — single-line installer for the Nexus lite binary
#
# Usage:
#   curl -fsSL https://get.nexus.chat | sh
#   # or with a specific version:
#   curl -fsSL https://get.nexus.chat | sh -s -- --version v0.9.5

set -eu

REPO="The-No-hands-Company/Nexus"
INSTALL_DIR="${NEXUS_INSTALL_DIR:-/usr/local/bin}"
VERSION=""

# ── Parse args ────────────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --dir)     INSTALL_DIR="$2"; shift 2 ;;
    *)         echo "Unknown option: $1"; exit 1 ;;
  esac
done

# ── Detect OS + arch ──────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  ASSET="nexus-linux-x86_64" ;;
      aarch64|arm64) ASSET="nexus-linux-aarch64" ;;
      *) echo "Unsupported Linux architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)  ASSET="nexus-macos-x86_64" ;;
      arm64)   ASSET="nexus-macos-aarch64" ;;
      *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    ASSET="nexus-windows-x86_64.exe"
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

# ── Resolve download URL ──────────────────────────────────────────────────────
if [ -z "$VERSION" ]; then
  URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
  URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

# ── Determine binary name ─────────────────────────────────────────────────────
case "$OS" in
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    BIN_NAME="nexus.exe"
    ;;
  *)
    BIN_NAME="nexus"
    ;;
esac

DEST="${INSTALL_DIR}/${BIN_NAME}"

# ── Download ──────────────────────────────────────────────────────────────────
echo "Downloading Nexus from ${URL} ..."

if command -v curl > /dev/null 2>&1; then
  curl -fsSL "$URL" -o "$DEST"
elif command -v wget > /dev/null 2>&1; then
  wget -q "$URL" -O "$DEST"
else
  echo "Error: curl or wget is required." >&2
  exit 1
fi

chmod +x "$DEST"

# ── Verify ────────────────────────────────────────────────────────────────────
INSTALLED_VERSION="$("$DEST" --version 2>/dev/null || echo '(unknown)')"
echo ""
echo "✅  Nexus installed to ${DEST}"
echo "    Version: ${INSTALLED_VERSION}"
echo ""
echo "Run your server:"
echo "    nexus serve --lite"
echo ""
echo "That's it — no Docker, no database setup required."
echo "Your server will be available at http://localhost:8080"
