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

# os detection and systemd validation
if [[ ! -f /etc/os-release ]]; then
    err "cannot detect operating system (missing /etc/os-release)."
fi

OS_TYPE=$(grep -w "ID" /etc/os-release | cut -d "=" -f 2 | tr -d '"')

if [[ "$OS_TYPE" = "manjaro" || "$OS_TYPE" = "manjaro-arm" || "$OS_TYPE" = "endeavouros" || "$OS_TYPE" = "cachyos" ]]; then
    OS_TYPE="arch"
elif [[ "$OS_TYPE" = "fedora-asahi-remix" ]]; then
    OS_TYPE="fedora"
elif [[ "$OS_TYPE" = "pop" || "$OS_TYPE" = "linuxmint" || "$OS_TYPE" = "zorin" ]]; then
    OS_TYPE="ubuntu"
fi

case "$OS_TYPE" in
    arch | ubuntu | debian | raspbian | centos | fedora | rhel | ol | rocky | sles | opensuse-leap | opensuse-tumbleweed | almalinux | amzn | alpine | postmarketos | tencentos) ;;
    *)
        err "operating system '$OS_TYPE' is not supported. this script only supports Debian, RedHat, Arch Linux, Alpine Linux, or SLES based operating systems."
        ;;
esac

if ! command -v systemctl >/dev/null 2>&1; then
    err "systemd (systemctl) is required to run slasha on the host, but was not found."
fi

# dependency checks

# curl/wget
if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    case "$OS_TYPE" in
        arch) pacman -Sy --noconfirm curl ;;
        alpine|postmarketos) apk add curl ;;
        ubuntu|debian|raspbian) apt-get update -y && apt-get install -y curl ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if command -v dnf >/dev/null 2>&1; then
                dnf install -y curl
            else
                yum install -y curl
            fi
            ;;
        sles|opensuse-leap|opensuse-tumbleweed) zypper install -y curl ;;
    esac
fi
if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    err "failed to install curl/wget."
fi

# ufw
if ! command -v ufw >/dev/null 2>&1; then
    case "$OS_TYPE" in
        arch) pacman -Sy --noconfirm ufw ;;
        alpine|postmarketos) apk add ufw ;;
        ubuntu|debian|raspbian)
            apt-get update -y && apt-get install -y ufw
            ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if [[ "$OS_TYPE" = "fedora" ]]; then
                dnf install -y ufw
            else
                dnf install -y epel-release || yum install -y epel-release || true
                if command -v dnf >/dev/null 2>&1; then
                    dnf install -y ufw
                else
                    yum install -y ufw
                fi
            fi
            ;;
        sles|opensuse-leap|opensuse-tumbleweed)
            zypper install -y ufw
            ;;
    esac
fi
if ! command -v ufw >/dev/null 2>&1; then
    err "failed to install ufw. please install it manually."
fi

# python3
if ! command -v python3 >/dev/null 2>&1; then
    case "$OS_TYPE" in
        arch) pacman -Sy --noconfirm python3 ;;
        alpine|postmarketos) apk add python3 ;;
        ubuntu|debian|raspbian)
            apt-get update -y && apt-get install -y python3
            ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if command -v dnf >/dev/null 2>&1; then
                dnf install -y python3
            else
                yum install -y python3
            fi
            ;;
        sles|opensuse-leap|opensuse-tumbleweed)
            zypper install -y python3
            ;;
    esac
fi
if ! command -v python3 >/dev/null 2>&1; then
    err "failed to install python3. please install it manually."
fi

# docker, docker compose and docker buildx
if [[ -x "$(command -v snap 2>/dev/null)" ]]; then
    SNAP_DOCKER_INSTALLED=$(snap list docker >/dev/null 2>&1 && echo "true" || echo "false")
    if [[ "$SNAP_DOCKER_INSTALLED" = "true" ]]; then
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
            systemctl enable docker.service >/dev/null 2>&1 || true
            systemctl start docker.service >/dev/null 2>&1 || true
            ;;
        amzn)
            dnf install docker -y
            DOCKER_CONFIG=${DOCKER_CONFIG:-/usr/local/lib/docker}
            mkdir -p $DOCKER_CONFIG/cli-plugins >/dev/null 2>&1
            curl -fsSL "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o $DOCKER_CONFIG/cli-plugins/docker-compose
            chmod +x $DOCKER_CONFIG/cli-plugins/docker-compose
            systemctl enable docker >/dev/null 2>&1 || true
            systemctl start docker >/dev/null 2>&1 || true
            ;;
        rocky)
            dnf install -y dnf-plugins-core || true
            dnf config-manager --add-repo https://download.docker.com/linux/rhel/docker-ce.repo || true
            dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            systemctl enable docker >/dev/null 2>&1 || true
            systemctl start docker >/dev/null 2>&1 || true
            ;;
        almalinux|tencentos)
            dnf install -y dnf-plugins-core || true
            dnf config-manager --add-repo=https://download.docker.com/linux/centos/docker-ce.repo || true
            dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin docker-buildx-plugin
            systemctl enable docker >/dev/null 2>&1 || true
            systemctl start docker >/dev/null 2>&1 || true
            ;;
        ubuntu|debian|raspbian|centos|fedora|rhel|sles|opensuse-leap|opensuse-tumbleweed)
            curl -fsSL https://get.docker.com | sh || true
            if ! command -v docker >/dev/null 2>&1; then
                if [[ "$OS_TYPE" =~ ^(ubuntu|debian|raspbian)$ ]]; then
                    apt-get update -y
                    apt-get install -y ca-certificates curl
                    install -m 0755 -d /etc/apt/keyrings
                    curl -fsSL https://download.docker.com/linux/$OS_TYPE/gpg -o /etc/apt/keyrings/docker.asc
                    chmod a+r /etc/apt/keyrings/docker.asc
                    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/$OS_TYPE $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | tee /etc/apt/sources.list.d/docker.list >/dev/null
                    apt-get update -y
                    apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
                fi
            fi
            if command -v systemctl >/dev/null 2>&1; then
                systemctl enable docker >/dev/null 2>&1 || true
                systemctl start docker >/dev/null 2>&1 || true
            fi
            ;;
    esac
fi

if ! command -v docker >/dev/null 2>&1; then
    err "failed to install docker. please install docker manually."
fi
if ! docker compose version >/dev/null 2>&1; then
    err "failed to install docker compose plugin. please install it manually."
fi
if ! docker buildx version >/dev/null 2>&1; then
    err "failed to install docker buildx plugin. please install it manually."
fi

# verify minimum docker version
MIN_DOCKER_VERSION=24
INSTALLED_DOCKER_VERSION=$(docker version --format '{{.Server.Version}}' 2>/dev/null | cut -d. -f1 || true)
if [[ -n "$INSTALLED_DOCKER_VERSION" ]] && [[ "$INSTALLED_DOCKER_VERSION" -lt "$MIN_DOCKER_VERSION" ]]; then
    err "docker version is too old ($INSTALLED_DOCKER_VERSION). slasha requires docker $MIN_DOCKER_VERSION or newer."
fi

# proxy
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
    if docker exec slasha-proxy wget -qO- http://127.0.0.1:2019/config/ >/dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

for i in $(seq 1 10); do
    if docker exec slasha-proxy test -f /data/caddy/pki/authorities/local/root.crt; then
        break
    fi
    sleep 0.5
done

echo "---BEGIN ROOT CA---"
docker exec slasha-proxy cat /data/caddy/pki/authorities/local/root.crt || true
echo "---END ROOT CA---"

# firewall
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

echo "slasha-node-setup: done"
