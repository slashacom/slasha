#!/usr/bin/env bash
#
# PID 1 inside the slasha container.
# Runs sshd (port 2222, for git push) and slasha-server side by side.

set -euo pipefail

# Match the in-container 'docker' group GID to the host's, so the slasha user
# can talk to the bind-mounted /var/run/docker.sock. The host's GID varies
# between distros, so we read it at runtime.
if [[ -S /var/run/docker.sock ]]; then
  sock_gid=$(stat -c '%g' /var/run/docker.sock)
  if getent group docker >/dev/null; then
    groupmod -g "$sock_gid" docker >/dev/null
  else
    groupadd -g "$sock_gid" docker >/dev/null
  fi
  if ! id -nG slasha | grep -qw docker; then
    usermod -aG docker slasha
  fi
fi

# Persist sshd host keys in the slasha-data named volume. Without this, every
# image rebuild would generate fresh keys and clients would see "host key
# changed" warnings on git push — training people to ignore those warnings is
# a real MITM risk.
HOST_KEY_DIR=/home/slasha/.slasha/sshd-host-keys
mkdir -p "$HOST_KEY_DIR"
chmod 700 "$HOST_KEY_DIR"
for type in rsa ecdsa ed25519; do
  key="$HOST_KEY_DIR/ssh_host_${type}_key"
  if [[ ! -f "$key" ]]; then
    ssh-keygen -q -t "$type" -N "" -f "$key" -C "slasha-host-${type}"
  fi
done
chown -R slasha:slasha "$HOST_KEY_DIR"

# sshd reads /etc/ssh/sshd_config.d/*.conf in addition to the main config.
{
  echo "Port 2222"
  echo "AllowUsers slasha"
  echo "PermitRootLogin no"
  echo "PasswordAuthentication no"
  echo "ChallengeResponseAuthentication no"
  echo "KbdInteractiveAuthentication no"
  echo "PubkeyAuthentication yes"
  for type in rsa ecdsa ed25519; do
    echo "HostKey $HOST_KEY_DIR/ssh_host_${type}_key"
  done
} > /etc/ssh/sshd_config.d/slasha.conf

# Run sshd as root in the background.
/usr/sbin/sshd -D &

# Drop privileges and run the HTTP server in the foreground.
exec runuser -u slasha -- /usr/local/bin/slasha serve
