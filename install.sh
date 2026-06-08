#!/bin/sh
#
# Install the slasha CLI/server binary on a Linux server.
#
#   curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/install.sh | sh
#
# Downloads the latest release binary from GitHub, verifies its checksum, and
# installs it to /usr/local/bin/slasha. Once installed, run `slasha serve` to
# start the server (Docker must be installed and running).
#
# Environment overrides:
#   SLASHA_VERSION      tag to install (e.g. v0.2.0). Default: latest release.
#   SLASHA_INSTALL_DIR  install directory. Default: /usr/local/bin.

set -eu

REPO="slashacom/slasha"
BIN="slasha"
INSTALL_DIR="${SLASHA_INSTALL_DIR:-/usr/local/bin}"
VERSION="${SLASHA_VERSION:-latest}"

err() {
  echo "slasha install: $*" >&2
  exit 1
}

# --- platform detection ---------------------------------------------------
os="$(uname -s)"
[ "$os" = "Linux" ] || err "only Linux servers are supported (detected: $os).
For macOS/Windows use the CLI from your package manager or build from source."

case "$(uname -m)" in
  x86_64 | amd64) target="x86_64-unknown-linux-gnu" ;;
  aarch64 | arm64) target="aarch64-unknown-linux-gnu" ;;
  *) err "unsupported architecture: $(uname -m) (need x86_64 or arm64)." ;;
esac

# --- http helper ----------------------------------------------------------
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1"; }
  fetch_to() { curl -fsSL -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO- "$1"; }
  fetch_to() { wget -qO "$2" "$1"; }
else
  err "need curl or wget installed."
fi

# --- resolve the release tag ----------------------------------------------
if [ "$VERSION" = "latest" ]; then
  api="https://api.github.com/repos/$REPO/releases/latest"
  if [ -n "${GITHUB_TOKEN:-}" ] && command -v curl >/dev/null 2>&1; then
    body="$(curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" "$api")"
  else
    body="$(fetch "$api")"
  fi
  tag="$(printf '%s' "$body" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name"[^"]*"([^"]+)".*/\1/')"
  [ -n "$tag" ] || err "could not determine the latest release tag from GitHub."
else
  tag="$VERSION"
fi

asset="slasha-$target.tar.gz"
base="https://github.com/$REPO/releases/download/$tag"

echo "slasha install: $tag ($target) -> $INSTALL_DIR/$BIN"

# --- download into a temp dir ---------------------------------------------
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

fetch_to "$base/$asset" "$tmp/$asset" || err "download failed: $base/$asset"

# --- verify checksum ------------------------------------------------------
if fetch_to "$base/SHA256SUMS" "$tmp/SHA256SUMS" 2>/dev/null; then
  expected="$(grep " $asset\$" "$tmp/SHA256SUMS" | awk '{print $1}')"
  [ -n "$expected" ] || err "no checksum for $asset in SHA256SUMS."

  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$asset" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')"
  else
    actual=""
  fi

  if [ -n "$actual" ] && [ "$actual" != "$expected" ]; then
    err "checksum mismatch for $asset (expected $expected, got $actual)."
  fi
else
  echo "slasha install: SHA256SUMS not found, skipping checksum verification." >&2
fi

# --- extract --------------------------------------------------------------
tar -xzf "$tmp/$asset" -C "$tmp"
[ -f "$tmp/$BIN" ] || err "archive did not contain a '$BIN' binary."
chmod +x "$tmp/$BIN"

# --- install (escalate only if needed) ------------------------------------
sudo=""
if [ ! -d "$INSTALL_DIR" ] || [ ! -w "$INSTALL_DIR" ]; then
  if [ "$(id -u)" -ne 0 ]; then
    command -v sudo >/dev/null 2>&1 || err "$INSTALL_DIR is not writable and sudo is unavailable. Re-run as root or set SLASHA_INSTALL_DIR."
    sudo="sudo"
  fi
fi

$sudo mkdir -p "$INSTALL_DIR"
$sudo install -m 0755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"

# --- report ---------------------------------------------------------------
echo "slasha install: installed $("$INSTALL_DIR/$BIN" --version 2>/dev/null || echo "$BIN $tag")"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "slasha install: note — $INSTALL_DIR is not on your PATH." >&2 ;;
esac

if ! command -v docker >/dev/null 2>&1; then
  echo "slasha install: note — 'slasha serve' requires Docker, which was not found on this host." >&2
fi

echo "slasha install: done. Run 'slasha serve' to start the server."
