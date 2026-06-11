#!/usr/bin/env bash
#
# install the slasha cli on linux or macos.
#
#   curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/scripts/install.sh | bash
#
# environment overrides:
#   SLASHA_VERSION      tag to install - default: latest.
#   SLASHA_INSTALL_DIR  install directory - default: ~/.local/bin.

set -euo pipefail

REPO="slashacom/slasha"
BIN="slasha"
INSTALL_DIR="${SLASHA_INSTALL_DIR:-$HOME/.local/bin}"

COLOR_OFF=''
COLOR_RED=''
COLOR_GREEN=''
COLOR_DIM=''
COLOR_YELLOW=''

if [[ -t 1 ]]; then
    COLOR_OFF='\033[0m'
    COLOR_RED='\033[0;31m'
    COLOR_GREEN='\033[0;32m'
    COLOR_DIM='\033[0;2m'
    COLOR_YELLOW='\033[0;33m'
fi

err()     { echo -e "${COLOR_RED}error${COLOR_OFF}: $*" >&2; exit 1; }
info()    { echo -e "${COLOR_DIM}$*${COLOR_OFF}"; }
success() { echo -e "${COLOR_GREEN}$*${COLOR_OFF}"; }
warn()    { echo -e "${COLOR_YELLOW}$*${COLOR_OFF}"; }

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64 | amd64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
                *) err "unsupported linux architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64)  echo "aarch64-apple-darwin" ;;
                *) err "unsupported macos architecture: $arch" ;;
            esac
            ;;
        *)
            err "unsupported os: $os"
            ;;
    esac
}

get_latest_version() {
    local body tag
    if command -v curl &>/dev/null; then
        body="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")"
    elif command -v wget &>/dev/null; then
        body="$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest")"
    else
        err "curl or wget is required."
    fi
    tag="$(printf '%s' "$body" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name"[^"]*"([^"]+)".*/\1/')"
    [[ -n "$tag" ]] || err "could not determine the latest release tag."
    echo "$tag"
}

tmpdir=""
cleanup() { [[ -n "$tmpdir" ]] && rm -rf "$tmpdir"; }
trap cleanup EXIT

download_and_install() {
    local tag="$1" target="$2"
    local asset="slasha-$target.tar.gz"
    local base="https://github.com/$REPO/releases/download/$tag"

    tmpdir="$(mktemp -d)"

    info "downloading $BIN $tag for $target..."
    if command -v curl &>/dev/null; then
        curl -fsSL "$base/$asset" -o "$tmpdir/$asset" || err "download failed: $base/$asset"
    else
        wget -q "$base/$asset" -O "$tmpdir/$asset" || err "download failed: $base/$asset"
    fi

    info "verifying checksum..."
    if curl -fsSL "$base/SHA256SUMS" -o "$tmpdir/SHA256SUMS" 2>/dev/null || \
       wget -q "$base/SHA256SUMS" -O "$tmpdir/SHA256SUMS" 2>/dev/null; then
        local expected actual
        expected="$(grep " $asset\$" "$tmpdir/SHA256SUMS" | awk '{print $1}')"
        [[ -n "$expected" ]] || err "no checksum entry for $asset in SHA256SUMS."
        if command -v sha256sum &>/dev/null; then
            actual="$(sha256sum "$tmpdir/$asset" | awk '{print $1}')"
        elif command -v shasum &>/dev/null; then
            actual="$(shasum -a 256 "$tmpdir/$asset" | awk '{print $1}')"
        fi
        [[ -z "${actual:-}" ]] || [[ "$actual" == "$expected" ]] || \
            err "checksum mismatch (expected $expected, got $actual)."
    else
        warn "SHA256SUMS not available — skipping checksum verification."
    fi

    info "extracting..."
    tar -xzf "$tmpdir/$asset" -C "$tmpdir"
    [[ -f "$tmpdir/$BIN" ]] || err "binary '$BIN' not found in archive."

    info "installing to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$tmpdir/$BIN" "$INSTALL_DIR/$BIN"
}

is_upgrade=false
installed_version=""
if [[ -x "$INSTALL_DIR/$BIN" ]]; then
    is_upgrade=true
    installed_version="$("$INSTALL_DIR/$BIN" --version 2>/dev/null | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+' || true)"
    info "existing installation found: $INSTALL_DIR/$BIN${installed_version:+ (v$installed_version)}"
fi

if [[ -n "${SLASHA_VERSION:-}" ]]; then
    tag="$SLASHA_VERSION"
else
    info "fetching latest version..."
    tag="$(get_latest_version)"
fi

info "latest version: $tag"

if [[ "$is_upgrade" == true && "${installed_version:-}" == "${tag#v}" ]]; then
    success "$BIN is already up to date (v${installed_version})."
    exit 0
fi

info "detecting platform..."
target="$(detect_target)"
info "target: $target"

if [[ "$is_upgrade" == true ]]; then
    info "upgrading $BIN to $tag..."
else
    info "installing $BIN $tag..."
fi

download_and_install "$tag" "$target"

if [[ "$is_upgrade" == true ]]; then
    success "$BIN upgraded to $tag successfully!"
else
    success "$BIN $tag installed successfully!"
fi

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) warn "note: add $INSTALL_DIR to your PATH (e.g. export PATH=\"\$PATH:$INSTALL_DIR\")" ;;
esac

info "run '$BIN --help' to get started."
