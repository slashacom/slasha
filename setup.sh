#!/usr/bin/env bash
#
# set up slasha on a fresh linux server.
#
#   curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/scripts/setup.sh | bash
#

set -euo pipefail

INSTALL_URL="https://raw.githubusercontent.com/slashacom/slasha/main/install.sh"
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

# --- dependency checks ----------------------------------------------------
header "checking dependencies"

if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    err "curl or wget is required but neither was found."
fi
success "curl/wget"

if ! command -v docker >/dev/null 2>&1; then
    err "docker is required but not installed."
fi
success "docker $(docker --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')"

if ! docker compose version >/dev/null 2>&1; then
    err "docker compose is required but not found."
fi
success "docker compose"

if ! command -v ufw >/dev/null 2>&1; then
    err "ufw is required but not installed."
fi
success "ufw"

if ! command -v sshd >/dev/null 2>&1; then
    err "openssh-server is required but not installed."
fi
if ! $SUDO systemctl is-active --quiet ssh 2>/dev/null && \
   ! $SUDO systemctl is-active --quiet sshd 2>/dev/null; then
    info "sshd is not running — starting it..."
    $SUDO systemctl enable --now ssh 2>/dev/null || $SUDO systemctl enable --now sshd 2>/dev/null || \
        err "failed to start sshd. start it manually and re-run."
fi
success "sshd"

# --- install binary -------------------------------------------------------
header "installing slasha"

if command -v curl >/dev/null 2>&1; then
    SLASHA_INSTALL_DIR=/usr/local/bin bash <(curl -fsSL "$INSTALL_URL")
else
    SLASHA_INSTALL_DIR=/usr/local/bin bash <(wget -qO- "$INSTALL_URL")
fi

# --- create system user ---------------------------------------------------
header "setting up system user"

if ! id slasha >/dev/null 2>&1; then
    $SUDO useradd --system --no-create-home --shell /sbin/nologin slasha
    success "created user: slasha"
else
    success "user slasha already exists"
fi

if ! groups slasha | grep -qw docker; then
    $SUDO usermod -aG docker slasha
    success "added slasha to docker group"
fi

# --- collect configuration ------------------------------------------------
header "configuration"

while true; do
    ask "platform domain (e.g. slasha.example.com): " domain
    [[ -n "${domain:-}" ]] && break
    warn "domain cannot be empty."
done

auto_secret="$(LC_ALL=C tr -dc 'a-f0-9' </dev/urandom | head -c 64)"
echo -e "\n  ${COLOR_DIM}auto-generated jwt secret:${COLOR_OFF}"
echo -e "  ${COLOR_DIM}$auto_secret${COLOR_OFF}\n"
ask "press enter to use it, or paste a custom 64-char hex secret: " custom_secret
jwt_secret="${custom_secret:-$auto_secret}"
[[ ${#jwt_secret} -ge 32 ]] || err "jwt secret must be at least 32 characters."

# --- write env file -------------------------------------------------------
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

# --- systemd service ------------------------------------------------------
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

# --- ufw ------------------------------------------------------------------
header "configuring firewall"

$SUDO ufw default deny incoming  >/dev/null
$SUDO ufw default allow outgoing >/dev/null
$SUDO ufw allow 22/tcp   comment "host ssh"    >/dev/null
$SUDO ufw allow 80/tcp   comment "http (acme)" >/dev/null
$SUDO ufw allow 443/tcp  comment "https"       >/dev/null
$SUDO ufw allow 2222/tcp comment "slasha git"  >/dev/null

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
success "firewall configured (22, 80, 443, 2222)"

# --- fail2ban -------------------------------------------------------------
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

# --- start service --------------------------------------------------------
header "starting slasha"

$SUDO systemctl start slasha
sleep 3

if $SUDO systemctl is-active --quiet slasha; then
    success "slasha is running"
else
    warn "slasha did not start cleanly."
    warn "check logs: sudo journalctl -u slasha -n 50"
fi

# --- summary --------------------------------------------------------------
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
