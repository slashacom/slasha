#!/usr/bin/env bash
#
# cleans up slasha configurations from a remote node by stopping and removing
# the proxy container, deleting slasha networks and volumes, and reverting
# the ufw firewall rules. It avoids uninstalling docker or ufw to prevent
# disrupting existing host services.

set -euo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
    echo "error: this script must run as root. configure your SSH key to log in as root (PermitRootLogin yes) or use a root-equivalent account."
    exit 1
fi

docker stop slasha-proxy >/dev/null 2>&1 || true
docker rm slasha-proxy >/dev/null 2>&1 || true
docker volume rm slasha-caddy-data slasha-caddy-config >/dev/null 2>&1 || true
docker network rm slasha-proxy >/dev/null 2>&1 || true

if command -v ufw >/dev/null 2>&1; then
    ufw delete allow 80/tcp >/dev/null 2>&1 || true
    ufw delete allow 443/tcp >/dev/null 2>&1 || true

    if grep -q "slasha: allow docker bridges" /etc/ufw/before.rules 2>/dev/null; then
        python3 - <<'PY'
src = open("/etc/ufw/before.rules").read()
marker = "-A ufw-before-input -i lo -j ACCEPT"
inject = marker + """

# slasha: allow docker bridges -> host
-A ufw-before-input -i docker0 -j ACCEPT
-A ufw-before-input -i br-+ -j ACCEPT
"""
open("/etc/ufw/before.rules", "w").write(src.replace(inject, marker, 1))
PY
        ufw reload >/dev/null 2>&1 || true
    fi
fi

echo "slasha-node-teardown: done"
