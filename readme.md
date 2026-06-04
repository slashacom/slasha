# Slasha

Slasha is a self-hosted platform for deploying apps to your own server. You push code with `git`, and Slasha builds it, runs it in a container, and serves it on a real domain with automatic HTTPS — your own private Heroku on a single Linux box.

## Contents

- [How it works](#how-it-works)
- [Requirements](#requirements)
- [Server setup](#server-setup)
- [Create the admin account](#create-the-admin-account)
- [Deploy an app](#deploy-an-app)
- [Managing apps](#managing-apps)
- [Data and backups](#data-and-backups)
- [Updating](#updating)
- [Uninstalling](#uninstalling)
- [Command reference](#command-reference)
- [Troubleshooting](#troubleshooting)
- [Security](#security)

## How it works

One binary runs the whole platform. `slasha serve` on the server:

- Serves the dashboard and API on port `3000`.
- Receives `git push`es into bare repositories.
- Builds each app into a Docker image and runs it as a container.
- Spawns a Caddy reverse proxy that routes traffic and issues Let's Encrypt certificates automatically.
- Provisions databases and caches (Postgres, MySQL, MongoDB, Redis) alongside your apps.

The same binary is your CLI client. You run `slasha create`, `slasha deploy`, and the rest from your own Linux machine, pointed at the server.

```
your machine ──CLI / git push──►  your server
                                    │
                                    ├─ slasha serve        dashboard + API  (:3000)
                                    ├─ Caddy proxy         auto HTTPS       (:80 / :443)
                                    │     ├─ example.com          → dashboard
                                    │     └─ myapp.example.com    → app container
                                    └─ Docker              builds + runs apps
```

State lives in one folder, `~/.slasha`: the SQLite database, git repos, and logs.

## Requirements

- A Linux server. A fresh Ubuntu 22.04 or 24.04 box is easiest. 2 GB RAM minimum.
- Root or `sudo` on that server.
- A domain you control, for HTTPS. You will point it and a wildcard at the server.
- Docker (installed in step 2).

You do not install Caddy, a database, or anything else by hand — Slasha manages those.

## Server setup

These steps use `example.com` and the IP `203.0.113.10`. Substitute your own throughout.

### 1. Point DNS at the server

Add two records:

| Type | Name            | Value          |
|------|-----------------|----------------|
| A    | `example.com`   | `203.0.113.10` |
| A    | `*.example.com` | `203.0.113.10` |

The wildcard is what gives every app its own subdomain (`myapp.example.com`) with no further DNS work.

> *Important* — if your DNS provider proxies traffic (for example Cloudflare's orange-cloud), turn it off for these records. Caddy must reach Let's Encrypt directly. Keep them as plain "DNS only" records.

### 2. Install Docker

```bash
curl -fsSL https://get.docker.com | sudo sh
sudo docker run --rm hello-world
```

### 3. Create the slasha user

Slasha runs as a dedicated `slasha` user. That same user is who people connect as to `git push`, so the name matters.

```bash
sudo useradd --create-home --shell /bin/bash slasha
sudo usermod -aG docker slasha
```

The `docker` group lets Slasha build and run containers without `sudo`.

### 4. Install the binary

The script detects your architecture, downloads the latest release, verifies its checksum, and installs to `/usr/local/bin/slasha`.

```bash
curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/install.sh | sh
slasha version
```

> *Tip* — re-run this any time to upgrade. To pin a version, prefix with `SLASHA_VERSION=v0.2.0`.

### 5. Configuration

Slasha reads its settings from environment variables. Write them to a file:

```bash
sudo mkdir -p /etc/slasha
sudo tee /etc/slasha/slasha.env >/dev/null <<EOF
SLASHA_ENV=production
SLASHA_PLATFORM_DOMAIN=example.com
JWT_SECRET=$(openssl rand -hex 32)
SLASHA_PORT=3000
EOF
sudo chmod 600 /etc/slasha/slasha.env
```

| Setting                  | Required | Meaning |
|--------------------------|----------|---------|
| `SLASHA_ENV`             | yes      | Use `production` on a real server — it enables Let's Encrypt HTTPS. `development` is for local testing only. |
| `SLASHA_PLATFORM_DOMAIN` | yes      | Your main domain. Serves the dashboard and gives each app a subdomain under it. |
| `JWT_SECRET`             | yes      | Signs login tokens. The command above generates one. Keep it private. |
| `SLASHA_PORT`            | no       | Dashboard/API port. Defaults to `3000`. |

> *Warning* — changing `JWT_SECRET` later invalidates every session and logs all users out.

### 6. Run as a service

```bash
sudo tee /etc/systemd/system/slasha.service >/dev/null <<'EOF'
[Unit]
Description=Slasha server
After=network-online.target docker.service
Requires=docker.service
Wants=network-online.target

[Service]
User=slasha
EnvironmentFile=/etc/slasha/slasha.env
ExecStart=/usr/local/bin/slasha serve
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now slasha
```

Verify and tail logs:

```bash
systemctl status slasha
journalctl -u slasha -f
```

> *Note* — the first start pulls the Caddy image and creates its network, so give it a few seconds. Look for `Slasha server starting on http://0.0.0.0:3000`.

### 7. Open the firewall

```bash
sudo ufw allow 22/tcp    # SSH: admin login and git push
sudo ufw allow 80/tcp    # HTTP: HTTPS certificate issuance
sudo ufw allow 443/tcp   # HTTPS: dashboard and apps
```

Caddy runs in a container and reaches the dashboard back on the host across Docker's bridge, which `ufw`'s default deny blocks. Allow bridge traffic by adding these lines to `/etc/ufw/before.rules`, right after the existing `-A ufw-before-input -i lo -j ACCEPT`:

```
-A ufw-before-input -i docker0 -j ACCEPT
-A ufw-before-input -i br-+ -j ACCEPT
```

```bash
sudo ufw enable
sudo ufw reload
```

> *Note* — port 3000 stays closed to the public. The dashboard is reached through your domain over HTTPS; Caddy talks to 3000 internally.

The server is ready. Open `https://example.com` and you should see the dashboard on a valid certificate.

## Create the admin account

Install the same binary on your own Linux machine (step 4) and point it at the server:

```bash
slasha set-url https://example.com
slasha login
```

The first login on a fresh server has no users yet, so it prompts you to create the admin account. That first account is the admin. Confirm:

```bash
slasha me
```

> *Note* — `slasha login` stores the token in your machine's keyring. On a headless box or in CI, skip login and pass the token directly with `export SLASHA_TOKEN=<token>`.

## Deploy an app

### Register your SSH key

```bash
slasha ssh-keys add --file ~/.ssh/id_ed25519.pub --title "my workstation"
```

### Create the app

```bash
slasha create myapp
```

This prints the app's git remote, of the form `slasha@example.com:myapp.git`.

### Push, then deploy

```bash
git remote add slasha slasha@example.com:myapp.git
git push -u slasha main
slasha deploy --app myapp
```

> *Important* — pushing only uploads code. Deployment is the separate `slasha deploy` step.

Watch the build, then visit the app:

```bash
slasha deployments logs --app myapp --follow
# → https://myapp.example.com
```

### How the build is chosen

- A `Dockerfile` in the repo is built as-is. Slasha reads `EXPOSE` for the port (default `8080`).
- No `Dockerfile`: the language is auto-detected and an image is built for you, listening on `8080`.
- A `Procfile` runs each process type it declares (for example `web` and `worker`).

> *Tip* — without a `Dockerfile`, make your app listen on `8080`, or add a `Dockerfile` with the correct `EXPOSE`.

## Managing apps

Run these from your machine. Pass `--app myapp`, or link the folder once with `slasha link` and drop the flag.

Environment variables:

```bash
slasha env set --app myapp DATABASE_URL=... SECRET_KEY=...
slasha env list --app myapp
```

Scale process types:

```bash
slasha scale --app myapp web=3 worker=1
```

Provision a database or cache, then reach it locally over a secure tunnel:

```bash
slasha provision --app myapp --kind postgres --name db --version 16
slasha proxy --app myapp db --port 5432
```

Add a custom domain (point its DNS at the server first). Slasha issues the certificate automatically:

```bash
slasha domains add --app myapp www.mysite.com
```

## Data and backups

Everything lives under the `slasha` user's home:

| Path                                | Holds |
|-------------------------------------|-------|
| `/home/slasha/.slasha/slasha.db`    | Database — users, apps, settings |
| `/home/slasha/.slasha/repos/`       | App git repositories |
| `/home/slasha/.slasha/logs/`        | Build and run logs |
| `/home/slasha/.ssh/authorized_keys` | Managed push keys — do not edit by hand |

Back up by copying `.slasha` while the service is stopped:

```bash
sudo systemctl stop slasha
sudo tar czf slasha-backup.tar.gz -C /home/slasha .slasha
sudo systemctl start slasha
```

> *Note* — HTTPS certificates live in a separate Caddy volume and are re-issued automatically, so they need no backup.

## Updating

```bash
curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/install.sh | sh
sudo systemctl restart slasha
```

Data, apps, and certificates are untouched by an upgrade.

## Uninstalling

```bash
sudo systemctl disable --now slasha
sudo rm /etc/systemd/system/slasha.service /usr/local/bin/slasha
sudo rm -rf /etc/slasha
```

To also wipe all data and running app containers:

```bash
sudo docker ps -aq --filter label=slasha.managed=true | xargs -r sudo docker rm -f
sudo userdel -r slasha
```

## Command reference

Run `slasha <command> --help` for details. Add `--diagnostic` to any command for version and system info.

| Command                   | Does |
|---------------------------|------|
| `slasha serve`            | Run the server (on the host, via the service). |
| `slasha set-url <url>`    | Point the CLI at your server. |
| `slasha login` / `logout` | Sign in (creates the admin on first use) or out. |
| `slasha me`               | Show the current user. |
| `slasha status`           | Check server health. |
| `slasha create <name>`    | Create an app. |
| `slasha list`             | List apps. |
| `slasha info`             | Show an app's details and git remotes. |
| `slasha link`             | Link the current folder to an app. |
| `slasha delete`           | Delete an app and everything it owns. |
| `slasha deploy`           | Build and release the latest pushed code. |
| `slasha deployments ...`  | List, stop, restart, redeploy, delete, or tail deployments. |
| `slasha env ...`          | List, set, or unset app env vars. |
| `slasha scale web=2 ...`  | Run more or fewer copies of a process type. |
| `slasha provision ...`    | Add a database or cache. |
| `slasha services ...`     | Manage attached services, their logs, and env vars. |
| `slasha proxy <service>`  | Tunnel a service to a local port over HTTPS. |
| `slasha domains ...`      | Add, list, or remove custom domains. |
| `slasha ssh-keys ...`     | Add, list, or remove push keys. |
| `slasha users ...`        | Admin: add, update, list, or remove users. |
| `slasha version`          | Print version information. |

## Troubleshooting

Dashboard does not load at your domain:

- `systemctl status slasha` — is the service running?
- `dig +short example.com` — does DNS return the server's IP?
- Ports 80 and 443 must be free; another `nginx`/`apache` will conflict with Caddy.

HTTPS certificate is missing or untrusted:

- Confirm `SLASHA_ENV=production`, then restart the service.
- Issuance needs port 80 reachable and DNS pointing at the server. Behind a CDN, switch records to "DNS only".

`git push` is rejected or asks for a password:

- The key must be registered with `slasha ssh-keys add`, and the remote must be the `slasha@...` one (not HTTPS).
- The remote user must be `slasha`, as in `slasha@example.com:myapp.git`.

Builds fail immediately:

- `groups slasha` must list `docker`. If you just added it, restart the service.
- Inspect output with `slasha deployments logs --app <name> --follow`.

App deployed but the URL errors:

- The app must listen on `8080`, unless its `Dockerfile` says otherwise via `EXPOSE`.
- Check the app's logs in the dashboard or via the deployments logs command.

## Security

- `JWT_SECRET` signs login tokens. Keep `/etc/slasha/slasha.env` root-only and never commit it.
- Deploys connect over SSH as `slasha`, but the managed `authorized_keys` restricts every key to Slasha's git handler — a key cannot open a shell.
- Keep the host patched. On Ubuntu: `sudo apt-get install -y unattended-upgrades`.
- Only the admin can manage users. Add more with `slasha users create`.
