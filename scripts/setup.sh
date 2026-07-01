#!/usr/bin/env bash
#
# set up slasha on a fresh linux server.
#
#   curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/scripts/setup.sh | bash
#

set -euo pipefail

INSTALL_URL="https://raw.githubusercontent.com/slashacom/slasha/main/scripts/install.sh"
DATA_DIR="/var/lib/slasha"
CONF_DIR="/etc/slasha"
SERVICE_FILE="/etc/systemd/system/slasha.service"

COLOR_OFF=''
COLOR_RED=''
COLOR_GREEN=''
COLOR_DIM=''
COLOR_YELLOW=''
COLOR_BOLD=''

if [[ -t 1 ]]; then
    COLOR_OFF='\033[0m'
    COLOR_RED='\033[0;31m'
    COLOR_GREEN='\033[0;32m'
    COLOR_DIM='\033[0;2m'
    COLOR_YELLOW='\033[0;33m'
    COLOR_BOLD='\033[1m'
fi

err()     { echo -e "${COLOR_RED}error${COLOR_OFF}: $*" >&2; exit 1; }
info()    { echo -e "  ${COLOR_DIM}$*${COLOR_OFF}"; }
success() { echo -e "  ${COLOR_GREEN}✓ $*${COLOR_OFF}"; }
warn()    { echo -e "  ${COLOR_YELLOW}! $*${COLOR_OFF}"; }
header()  { echo -e "\n${COLOR_BOLD}$*${COLOR_OFF}"; }
ask()     { read -rp "  $1" "$2" </dev/tty; }

SUDO=""
if [[ "$(id -u)" -ne 0 ]]; then
    command -v sudo >/dev/null 2>&1 || err "this script requires root or sudo."
    SUDO="sudo"
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

if [[ "$OS_TYPE" = "arch" || "$OS_TYPE" = "archarm" ]]; then
    OS_VERSION="rolling"
else
    OS_VERSION=$(grep -w "VERSION_ID" /etc/os-release | cut -d "=" -f 2 | tr -d '"' || echo "unknown")
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
header "checking dependencies"

# curl/wget
if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    info "neither curl nor wget found. installing curl..."
    case "$OS_TYPE" in
        arch) $SUDO pacman -Sy --noconfirm curl ;;
        alpine|postmarketos) $SUDO apk add curl ;;
        ubuntu|debian|raspbian) $SUDO apt-get update -y && $SUDO apt-get install -y curl ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if command -v dnf >/dev/null 2>&1; then
                $SUDO dnf install -y curl
            else
                $SUDO yum install -y curl
            fi
            ;;
        sles|opensuse-leap|opensuse-tumbleweed) $SUDO zypper install -y curl ;;
    esac
fi
if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    err "failed to install curl/wget."
fi
success "curl/wget"

# ufw
if ! command -v ufw >/dev/null 2>&1; then
    info "ufw is not installed. installing ufw..."
    case "$OS_TYPE" in
        arch) $SUDO pacman -Sy --noconfirm ufw ;;
        alpine|postmarketos) $SUDO apk add ufw ;;
        ubuntu|debian|raspbian)
            $SUDO apt-get update -y && $SUDO apt-get install -y ufw
            ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if [[ "$OS_TYPE" = "fedora" ]]; then
                $SUDO dnf install -y ufw
            else
                $SUDO dnf install -y epel-release || $SUDO yum install -y epel-release || true
                if command -v dnf >/dev/null 2>&1; then
                    $SUDO dnf install -y ufw
                else
                    $SUDO yum install -y ufw
                fi
            fi
            ;;
        sles|opensuse-leap|opensuse-tumbleweed)
            $SUDO zypper install -y ufw
            ;;
    esac
fi
if ! command -v ufw >/dev/null 2>&1; then
    err "failed to install ufw. please install it manually."
fi
success "ufw"

# sshd
if ! command -v sshd >/dev/null 2>&1; then
    info "sshd (openssh-server) is not installed. installing..."
    case "$OS_TYPE" in
        arch)
            $SUDO pacman -Sy --noconfirm openssh
            ;;
        alpine|postmarketos)
            $SUDO apk add openssh
            ;;
        ubuntu|debian|raspbian)
            $SUDO apt-get update -y && $SUDO apt-get install -y openssh-server
            ;;
        centos|fedora|rhel|ol|rocky|almalinux|amzn|tencentos)
            if command -v dnf >/dev/null 2>&1; then
                $SUDO dnf install -y openssh-server
            else
                $SUDO yum install -y openssh-server
            fi
            ;;
        sles|opensuse-leap|opensuse-tumbleweed)
            $SUDO zypper install -y openssh
            ;;
    esac
fi
if ! command -v sshd >/dev/null 2>&1; then
    err "failed to install openssh-server. please install it manually."
fi

if ! $SUDO systemctl is-active --quiet ssh 2>/dev/null && \
   ! $SUDO systemctl is-active --quiet sshd 2>/dev/null; then
    info "sshd is not running — starting it..."
    $SUDO systemctl enable --now ssh 2>/dev/null || $SUDO systemctl enable --now sshd 2>/dev/null || \
        err "failed to start sshd. start it manually and re-run."
fi
success "sshd"

# docker and docker compose
if [[ -x "$(command -v snap)" ]]; then
    SNAP_DOCKER_INSTALLED=$(snap list docker >/dev/null 2>&1 && echo "true" || echo "false")
    if [[ "$SNAP_DOCKER_INSTALLED" = "true" ]]; then
        err "docker is installed via snap. snap-based docker is not supported by slasha. please remove snap docker and run this script again."
    fi
fi

if ! command -v docker >/dev/null 2>&1 || ! docker compose version >/dev/null 2>&1 || ! docker buildx version >/dev/null 2>&1; then
    info "docker, docker compose or docker buildx is missing. installing docker..."
    case "$OS_TYPE" in
        alpine|postmarketos)
            $SUDO apk add docker docker-cli-compose docker-cli-buildx
            $SUDO rc-update add docker default >/dev/null 2>&1 || true
            $SUDO service docker start >/dev/null 2>&1 || true
            ;;
        arch)
            $SUDO pacman -Syu --noconfirm --needed docker docker-compose docker-buildx
            $SUDO systemctl enable docker.service >/dev/null 2>&1 || true
            $SUDO systemctl start docker.service >/dev/null 2>&1 || true
            ;;
        amzn)
            $SUDO dnf install docker -y
            DOCKER_CONFIG=${DOCKER_CONFIG:-/usr/local/lib/docker}
            $SUDO mkdir -p $DOCKER_CONFIG/cli-plugins >/dev/null 2>&1
            $SUDO curl -fsSL "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o $DOCKER_CONFIG/cli-plugins/docker-compose
            $SUDO chmod +x $DOCKER_CONFIG/cli-plugins/docker-compose
            $SUDO systemctl enable docker >/dev/null 2>&1 || true
            $SUDO systemctl start docker >/dev/null 2>&1 || true
            ;;
        rocky)
            $SUDO dnf install -y dnf-plugins-core || true
            $SUDO dnf config-manager --add-repo https://download.docker.com/linux/rhel/docker-ce.repo || true
            $SUDO dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            $SUDO systemctl enable docker >/dev/null 2>&1 || true
            $SUDO systemctl start docker >/dev/null 2>&1 || true
            ;;
        almalinux|tencentos)
            $SUDO dnf install -y dnf-plugins-core || true
            $SUDO dnf config-manager --add-repo=https://download.docker.com/linux/centos/docker-ce.repo || true
            $SUDO dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin docker-buildx-plugin
            $SUDO systemctl enable docker >/dev/null 2>&1 || true
            $SUDO systemctl start docker >/dev/null 2>&1 || true
            ;;
        ubuntu|debian|raspbian|centos|fedora|rhel|sles|opensuse-leap|opensuse-tumbleweed)
            curl -fsSL https://get.docker.com | sh || true
            if ! command -v docker >/dev/null 2>&1; then
                info "automated docker script failed. trying package manager..."
                if [[ "$OS_TYPE" =~ ^(ubuntu|debian|raspbian)$ ]]; then
                    $SUDO apt-get update -y
                    $SUDO apt-get install -y ca-certificates curl
                    $SUDO install -m 0755 -d /etc/apt/keyrings
                    $SUDO curl -fsSL https://download.docker.com/linux/$OS_TYPE/gpg -o /etc/apt/keyrings/docker.asc
                    $SUDO chmod a+r /etc/apt/keyrings/docker.asc
                    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/$OS_TYPE $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" | $SUDO tee /etc/apt/sources.list.d/docker.list >/dev/null
                    $SUDO apt-get update -y
                    $SUDO apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
                fi
            fi
            if command -v systemctl >/dev/null 2>&1; then
                $SUDO systemctl enable docker >/dev/null 2>&1 || true
                $SUDO systemctl start docker >/dev/null 2>&1 || true
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

success "docker $(docker --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')"
success "docker compose $(docker compose version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')"
success "docker buildx $(docker buildx version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')"

# railpack
if command -v railpack >/dev/null 2>&1 || [[ -x "/usr/local/bin/railpack" ]]; then
    success "railpack is already installed"
else
    info "railpack is missing. installing railpack..."
    if command -v curl >/dev/null 2>&1; then
        curl -sSL https://railpack.com/install.sh | $SUDO sh -s -- --bin-dir /usr/local/bin
    else
        wget -qO- https://railpack.com/install.sh | $SUDO sh -s -- --bin-dir /usr/local/bin
    fi
    
    if ! command -v railpack >/dev/null 2>&1 && ! [[ -x "/usr/local/bin/railpack" ]]; then
        err "failed to install railpack. please install it manually."
    fi
    success "railpack"
fi

# install binary
if command -v slasha >/dev/null 2>&1 || [[ -x "/usr/local/bin/slasha" ]]; then
    success "slasha is already installed"
else
    ask "slasha is not installed. do you want to run the automatic install script? (Y/n): " choice
    choice="${choice:-Y}"
    if [[ "$choice" =~ ^[Yy]$ ]]; then
        if command -v curl >/dev/null 2>&1; then
            SLASHA_INSTALL_DIR=/usr/local/bin bash <(curl -fsSL "$INSTALL_URL")
        else
            SLASHA_INSTALL_DIR=/usr/local/bin bash <(wget -qO- "$INSTALL_URL")
        fi
    else
        info "please install slasha manually and re-run this script."
        exit 0
    fi
fi

# create system user
header "setting up system user"

if ! id slasha >/dev/null 2>&1; then
    nologin="$(command -v nologin)"
    $SUDO useradd --system --home-dir /var/lib/slasha --no-create-home --shell "$nologin" slasha
    success "created user: slasha"
else
    $SUDO usermod -d /var/lib/slasha slasha
    success "user slasha already exists"
fi

if ! groups slasha | grep -qw docker; then
    $SUDO usermod -aG docker slasha
    success "added slasha to docker group"
fi

# collect configuration
header "configuration"

while true; do
    ask "platform domain (e.g. slasha.example.com): " domain
    [[ -n "${domain:-}" ]] && break
    warn "domain cannot be empty."
done

auto_secret="$(set +o pipefail; LC_ALL=C tr -dc 'a-f0-9' </dev/urandom | head -c 64)"
echo -e "\n  ${COLOR_DIM}auto-generated jwt secret:${COLOR_OFF}"
echo -e "  ${COLOR_DIM}$auto_secret${COLOR_OFF}\n"
ask "press enter to use it, or paste a custom 64-char hex secret: " custom_secret
jwt_secret="${custom_secret:-$auto_secret}"
[[ ${#jwt_secret} -ge 32 ]] || err "jwt secret must be at least 32 characters."

# write env file
header "writing configuration"

$SUDO mkdir -p "$CONF_DIR"
$SUDO tee "$CONF_DIR/.env" >/dev/null <<EOF
SLASHA_ENV=production
SLASHA_PLATFORM_DOMAIN=$domain
JWT_SECRET=$jwt_secret
EOF
$SUDO chmod 600 "$CONF_DIR/.env"
$SUDO chown slasha:slasha "$CONF_DIR/.env"
success "wrote $CONF_DIR/.env"

$SUDO mkdir -p "$DATA_DIR"
$SUDO chown slasha:slasha "$DATA_DIR"
success "created data directory: $DATA_DIR"

# systemd service
header "registering systemd service"

$SUDO tee "$SERVICE_FILE" >/dev/null <<'EOF'
[Unit]
Description=Slasha PaaS
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=slasha
Group=slasha
EnvironmentFile=/etc/slasha/.env
Environment="HOME=/var/lib/slasha"
ExecStart=/usr/local/bin/slasha serve
WorkingDirectory=/var/lib/slasha
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

$SUDO systemctl daemon-reload
$SUDO systemctl enable slasha >/dev/null
success "registered slasha.service"

# ufw
header "configuring firewall"

$SUDO ufw default deny incoming  >/dev/null
$SUDO ufw default allow outgoing >/dev/null
$SUDO ufw allow 22/tcp   comment "host ssh"    >/dev/null
$SUDO ufw allow 80/tcp   comment "http (acme)" >/dev/null
$SUDO ufw allow 443/tcp  comment "https"       >/dev/null

# allow docker bridges -> host so slasha (host process) can reach containers
# on docker0 and any custom br-* bridges.
if ! grep -q "slasha: allow docker bridges" /etc/ufw/before.rules 2>/dev/null; then
    $SUDO python3 - <<'PY'
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

$SUDO ufw --force enable >/dev/null
$SUDO ufw reload         >/dev/null
success "firewall configured (22, 80, 443)"

# fail2ban
header "configuring fail2ban"

if command -v fail2ban-server >/dev/null 2>&1; then
    $SUDO mkdir -p /etc/fail2ban/jail.d
    $SUDO tee /etc/fail2ban/jail.d/sshd.conf >/dev/null <<'EOF'
[sshd]
enabled  = true
port     = 22
backend  = systemd
maxretry = 5
findtime = 10m
bantime  = 1h
EOF
    $SUDO systemctl enable --now fail2ban >/dev/null
    $SUDO systemctl restart fail2ban
    success "fail2ban configured"
else
    warn "fail2ban not found — skipping"
fi

# start service
header "starting slasha"

$SUDO systemctl start slasha
sleep 3

if $SUDO systemctl is-active --quiet slasha; then
    success "slasha is running"
else
    warn "slasha did not start cleanly."
    warn "check logs: sudo journalctl -u slasha -n 50"
fi

# summary
server_ip="$(curl -4fsSL ifconfig.me 2>/dev/null || echo '<server-ip>')"

echo ""
echo -e "${COLOR_BOLD}setup complete!${COLOR_OFF}"
echo ""
echo -e "  ${COLOR_DIM}domain ${COLOR_OFF}  ${COLOR_GREEN}https://$domain${COLOR_OFF}"
echo -e "  ${COLOR_DIM}service${COLOR_OFF}  slasha.service"
echo -e "  ${COLOR_DIM}env    ${COLOR_OFF}  $CONF_DIR/.env"
echo -e "  ${COLOR_DIM}data   ${COLOR_OFF}  $DATA_DIR"
echo ""
echo -e "${COLOR_DIM}next steps:${COLOR_OFF}"
echo -e "  1. point your DNS A record:  $domain → $server_ip"
echo -e "  2. visit https://$domain"
echo ""
echo -e "${COLOR_DIM}useful commands:${COLOR_OFF}"
echo -e "  sudo systemctl status slasha"
echo -e "  sudo journalctl -u slasha -f"
echo -e "  sudo systemctl restart slasha"
echo ""
