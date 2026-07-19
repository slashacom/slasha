#!/usr/bin/env bash
#
# prepares a remote Linux server to act as a node

set -euo pipefail

err() { echo "error: $*" >&2; exit 1; }

# this script must run as root (no sudo fallback — it's meant to run
# non-interactively over SSH as root when a node is attached).
if [[ "$(id -u)" -ne 0 ]]; then
    err "this script must run as root. configure your SSH key to log in as root (PermitRootLogin yes) or use a root-equivalent account."
fi

if [[ "$(uname)" != "Linux" ]]; then
    err "only linux is supported"
fi

if [[ ! -f /etc/os-release ]]; then
    err "cannot detect operating system (missing /etc/os-release)."
fi

OS_TYPE=$(grep -w "ID" /etc/os-release | cut -d "=" -f 2 | tr -d '"')

case "$OS_TYPE" in
    manjaro|manjaro-arm|endeavouros|cachyos) OS_TYPE="arch" ;;
    fedora-asahi-remix)                      OS_TYPE="fedora" ;;
    pop|linuxmint|zorin)                     OS_TYPE="ubuntu" ;;
esac

case "$OS_TYPE" in
    arch | ubuntu | debian | raspbian | centos | fedora | rhel | ol | rocky | sles | opensuse-leap | opensuse-tumbleweed | almalinux | amzn | alpine | postmarketos | tencentos) ;;
    *)
        err "operating system '$OS_TYPE' is not supported. this script only supports Debian, RedHat, Arch Linux, Alpine Linux, or SLES based operating systems."
        ;;
esac

if ! command -v systemctl >/dev/null 2>&1; then
    err "systemd (systemctl) is required to run slasha on the host, but was not found."
fi

pkg_install() {
    case "$OS_TYPE" in
        arch)                          pacman -Sy --noconfirm "$@" ;;
        alpine|postmarketos)           apk add "$@" ;;
        ubuntu|debian|raspbian)        apt-get update -y && apt-get install -y "$@" ;;
        sles|opensuse-leap|opensuse-tumbleweed) zypper install -y "$@" ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if command -v dnf >/dev/null 2>&1; then
                dnf install -y "$@"
            else
                yum install -y "$@"
            fi
            ;;
    esac
}

# require <command-to-check> <package-name...>
require() {
    local cmd=$1; shift
    command -v "$cmd" >/dev/null 2>&1 && return 0
    pkg_install "$@"
    command -v "$cmd" >/dev/null 2>&1 || err "failed to install $cmd. please install it manually."
}

require curl curl
require ufw ufw
require python3 python3

if [[ -x "$(command -v snap 2>/dev/null)" ]]; then
    if snap list docker >/dev/null 2>&1; then
        err "docker is installed via snap. snap-based docker is not supported by slasha. please remove snap docker and run this script again."
    fi
fi

if ! command -v docker >/dev/null 2>&1 || ! docker compose version >/dev/null 2>&1 || ! docker buildx version >/dev/null 2>&1; then
    case "$OS_TYPE" in
        alpine|postmarketos)
            apk add docker docker-cli-compose docker-cli-buildx
            rc-update add docker default >/dev/null 2>&1 || true
            service docker start >/dev/null 2>&1 || true
            ;;
        arch)
            pacman -Syu --noconfirm --needed docker docker-compose docker-buildx
            systemctl enable --now docker.service >/dev/null 2>&1 || true
            ;;
        amzn)
            dnf install docker -y
            DOCKER_CONFIG=${DOCKER_CONFIG:-/usr/local/lib/docker}
            mkdir -p "$DOCKER_CONFIG/cli-plugins" >/dev/null 2>&1
            curl -fsSL "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o "$DOCKER_CONFIG/cli-plugins/docker-compose"
            chmod +x "$DOCKER_CONFIG/cli-plugins/docker-compose"
            systemctl enable --now docker >/dev/null 2>&1 || true
            ;;
        rocky|almalinux|tencentos)
            repo_os="rhel"; [[ "$OS_TYPE" != "rocky" ]] && repo_os="centos"
            dnf install -y dnf-plugins-core || true
            dnf config-manager --add-repo "https://download.docker.com/linux/${repo_os}/docker-ce.repo" || true
            dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            systemctl enable --now docker >/dev/null 2>&1 || true
            ;;
        ubuntu|debian|raspbian|centos|fedora|rhel|sles|opensuse-leap|opensuse-tumbleweed)
            curl -fsSL https://get.docker.com | sh || true
            if ! command -v docker >/dev/null 2>&1 && [[ "$OS_TYPE" =~ ^(ubuntu|debian|raspbian)$ ]]; then
                apt-get update -y
                apt-get install -y ca-certificates curl
                install -m 0755 -d /etc/apt/keyrings
                curl -fsSL "https://download.docker.com/linux/$OS_TYPE/gpg" -o /etc/apt/keyrings/docker.asc
                chmod a+r /etc/apt/keyrings/docker.asc
                echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/$OS_TYPE $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" \
                    | tee /etc/apt/sources.list.d/docker.list >/dev/null
                apt-get update -y
                apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            fi
            systemctl enable --now docker >/dev/null 2>&1 || true
            ;;
    esac
fi

command -v docker >/dev/null 2>&1 || err "failed to install docker. please install docker manually."
docker compose version >/dev/null 2>&1 || err "failed to install docker compose plugin. please install it manually."
docker buildx version >/dev/null 2>&1 || err "failed to install docker buildx plugin. please install it manually."

# verify minimum docker version
MIN_DOCKER_VERSION=24
INSTALLED_DOCKER_VERSION=$(docker version --format '{{.Server.Version}}' 2>/dev/null | cut -d. -f1 || true)
if [[ -n "$INSTALLED_DOCKER_VERSION" ]] && [[ "$INSTALLED_DOCKER_VERSION" -lt "$MIN_DOCKER_VERSION" ]]; then
    err "docker version is too old ($INSTALLED_DOCKER_VERSION). slasha requires docker $MIN_DOCKER_VERSION or newer."
fi

docker network create slasha-proxy >/dev/null 2>&1 || true
docker volume create slasha-caddy-data >/dev/null 2>&1 || true
docker volume create slasha-caddy-config >/dev/null 2>&1 || true

docker pull caddy:latest
docker run -d \
    --name slasha-proxy \
    --network slasha-proxy \
    --restart unless-stopped \
    -p 80:80 \
    -p 443:443 \
    -p 127.0.0.1:2019:2019 \
    -v slasha-caddy-data:/data \
    -v slasha-caddy-config:/config \
    -l slasha.managed=true \
    -l slasha.role=proxy \
    caddy:latest \
    /bin/sh -c "printf '{\n  admin 0.0.0.0:2019\n}\nlocalhost {\n  tls internal\n  respond \"ok\"\n}\n' > /etc/caddy/Caddyfile && caddy run --config /etc/caddy/Caddyfile --adapter caddyfile"

for i in $(seq 1 20); do
    docker exec slasha-proxy wget -qO- http://127.0.0.1:2019/config/ >/dev/null 2>&1 && break
    sleep 0.5
done

for i in $(seq 1 10); do
    docker exec slasha-proxy test -f /data/caddy/pki/authorities/local/root.crt && break
    sleep 0.5
done

echo "---BEGIN ROOT CA---"
docker exec slasha-proxy cat /data/caddy/pki/authorities/local/root.crt || true
echo "---END ROOT CA---"

ufw default deny incoming  >/dev/null 2>&1 || true
ufw default allow outgoing >/dev/null 2>&1 || true
ufw allow "${SSH_PORT:-22}/tcp" comment "host ssh" >/dev/null 2>&1 || true
ufw allow 80/tcp comment "http" >/dev/null 2>&1 || true
ufw allow 443/tcp comment "https" >/dev/null 2>&1 || true

# allow docker bridges -> host so slasha (host process) can reach containers
# on docker0 and any custom br-* bridges.
if ! grep -q "slasha: allow docker bridges" /etc/ufw/before.rules 2>/dev/null; then
    python3 - <<'PY'
src = open("/etc/ufw/before.rules").read()
marker = "-A ufw-before-input -i lo -j ACCEPT"
inject = marker + """

# slasha: allow docker bridges -> host
-A ufw-before-input -i docker0 -j ACCEPT
-A ufw-before-input -i br-+ -j ACCEPT
"""
open("/etc/ufw/before.rules", "w").write(src.replace(marker, inject, 1))
PY
fi

ufw --force enable >/dev/null 2>&1 || true
ufw reload >/dev/null 2>&1 || true