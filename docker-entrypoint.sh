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

# Run sshd as root in the background. openssh-server's postinstall has already
# generated host keys at image build time.
/usr/sbin/sshd -D &

# Drop privileges and run the HTTP server in the foreground.
exec runuser -u slasha -- /usr/local/bin/slasha-server
