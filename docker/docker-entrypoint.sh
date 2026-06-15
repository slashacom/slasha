#!/usr/bin/env bash

set -euo pipefail

SSH_KEY_TYPES=(rsa ecdsa ed25519)

# sync docker socket's gid and add slasha user to it
if [[ -S /var/run/docker.sock ]]; then
  sock_gid=$(stat -c '%g' /var/run/docker.sock)
  groupadd -g "$sock_gid" -f docker
  usermod -aG docker slasha
fi

# persist ssh host keys
HOST_KEY_DIR=/home/slasha/.slasha/sshd-host-keys
install -d -m 700 -o slasha -g slasha "$HOST_KEY_DIR"

for type in "${SSH_KEY_TYPES[@]}"; do
  key="$HOST_KEY_DIR/ssh_host_${type}_key"
  [[ -f "$key" ]] || ssh-keygen -q -t "$type" -N "" -f "$key"
done

# sshd config
{
  cat <<EOF
Port 2222
AllowUsers slasha
PermitRootLogin no
PasswordAuthentication no
ChallengeResponseAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
EOF

  for type in "${SSH_KEY_TYPES[@]}"; do
    echo "HostKey $HOST_KEY_DIR/ssh_host_${type}_key"
  done
} > /etc/ssh/sshd_config.d/slasha.conf

# start sshd as root in the background
/usr/sbin/sshd -D &

# run app as slasha user
exec runuser -u slasha -- /usr/local/bin/slasha serve