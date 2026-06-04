# Slasha

Slasha is a small, self-hosted platform for deploying apps to your own server. You push your code with `git`, and Slasha builds it, runs it in a container, and serves it on a real domain with automatic HTTPS. Think of it as your own private Heroku or Railway that runs on a single Linux box you control.

This guide walks you through setting up a server from scratch.

---

## How it works

You run one program on your server: the `slasha` binary. Started with `slasha serve`, it does everything:

- Serves a web dashboard and an API (default port `3000`).
- Receives your `git push`es into bare repositories.
- Builds each app into a Docker image and runs it as a container.
- Spawns a Caddy reverse proxy (in a container) that routes traffic to your apps and gets free HTTPS certificates from Let's Encrypt automatically.
- Can run databases and caches (Postgres, MySQL, MongoDB, Redis) next to your apps.

You manage everything from your own Linux machine using the same `slasha` binary as a command-line client (`slasha create`, `slasha deploy`, and so on).

```
your machine ──CLI / git push──►  your server
                                    │
                                    ├─ slasha serve        (dashboard + API on :3000)
                                    ├─ Caddy proxy         (:80 / :443, auto HTTPS)
                                    │     ├─ yourdomain.com        → dashboard
                                    │     └─ myapp.yourdomain.com  → your app container
                                    └─ Docker               (builds + runs your apps)
```

Everything Slasha stores lives in one folder: `~/.slasha` (the SQLite database, your git repos, and logs).

---

## What you need

- A Linux server (a fresh Ubuntu 22.04 or 24.04 box is the easiest). 2 GB of RAM is a reasonable minimum; more if you build heavy apps.
- Root or `sudo` access on that server.
- A domain name you control, so apps can get real HTTPS. You will point it (and a wildcard) at the server's IP.
- Docker installed on the server (step 2 below).

You do not need to install Caddy, a database, or anything else by hand. Slasha sets those up for you.

---

## Install

The steps below assume a fresh Ubuntu server and a domain called `example.com`. Replace `example.com` with your own domain and `203.0.113.10` with your server's public IP everywhere they appear.

### 1. Point your domain at the server

Add two DNS records at your DNS provider:

| Type | Name           | Value          |
|------|----------------|----------------|
| A    | `example.com`   | `203.0.113.10` |
| A    | `*.example.com` | `203.0.113.10` |

The first record points your main domain at the server (it serves the dashboard). The wildcard record is what lets every app get its own subdomain, like `myapp.example.com`, without you touching DNS again.

If your DNS provider has a proxy or CDN feature (for example Cloudflare's orange-cloud), turn it off for these records. Slasha's proxy needs to talk to Let's Encrypt directly to get certificates. Keep them as plain "DNS only" records.

DNS changes can take a few minutes to spread. You can continue with the next steps while you wait.

### 2. Install Docker

Slasha uses Docker to build and run your apps. On Ubuntu:

```bash
curl -fsSL https://get.docker.com | sudo sh
```

When it finishes, check it works:

```bash
sudo docker run --rm hello-world
```

### 3. Create a user for Slasha

Slasha runs as its own dedicated user named `slasha`. This is also the user people connect as when they `git push` to deploy, so the name matters.

```bash
sudo useradd --create-home --shell /bin/bash slasha
sudo usermod -aG docker slasha
```

The first line creates the user with a home directory. The second adds it to the `docker` group so Slasha can build and run containers without `sudo`.

### 4. Install the Slasha binary

Run the install script. It detects your architecture (x86_64 or arm64), downloads the latest release from GitHub, checks the file against its published checksum, and installs it to `/usr/local/bin/slasha`:

```bash
curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/install.sh | sh
```

Check it installed:

```bash
slasha version
```

You can re-run the same command any time to upgrade to the newest release. To pin a specific version instead of the latest, set `SLASHA_VERSION`:

```bash
curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/install.sh | SLASHA_VERSION=v0.2.0 sh
```

### 5. Configure Slasha

Slasha reads a few settings from environment variables. Create a config file for them:

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

What each setting means:

| Setting                  | Required | What it does |
|--------------------------|----------|--------------|
| `SLASHA_ENV`             | yes      | Set to `production` on a real server. This turns on real Let's Encrypt HTTPS certificates. Leave it as `development` only for local testing. |
| `SLASHA_PLATFORM_DOMAIN` | yes      | Your main domain. The dashboard is served here, and every app gets a subdomain under it (for example `myapp.example.com`). |
| `JWT_SECRET`             | yes      | A random secret used to sign login tokens. The command above generates one for you. Keep it private. Changing it later logs everyone out. |
| `SLASHA_PORT`            | no       | The port the dashboard and API listen on. Defaults to `3000`. You normally do not need to change this. |

### 6. Run Slasha as a service

Create a systemd service so Slasha starts on boot and restarts if it crashes:

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
```

Start it and tell it to run on boot:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now slasha
```

Check that it is running and watch the logs:

```bash
systemctl status slasha
journalctl -u slasha -f
```

On the first start, Slasha downloads the Caddy proxy image and sets up its network, so give it a few seconds. Once it is up, the logs show a line like `Slasha server starting on http://0.0.0.0:3000`.

### 7. Open the firewall

Your apps and dashboard are served over ports 80 and 443. Git push and admin SSH use port 22. If you use the `ufw` firewall, allow those:

```bash
sudo ufw allow 22/tcp    # SSH (admin login and git push)
sudo ufw allow 80/tcp    # HTTP (needed for HTTPS certificate issuance)
sudo ufw allow 443/tcp   # HTTPS (your dashboard and apps)
```

There is one extra step if you enable `ufw`. Slasha's Caddy proxy runs in a container and needs to reach the dashboard back on the host. With `ufw`'s default "deny incoming", that internal traffic gets blocked. Allow traffic coming from Docker's bridge networks into the host by adding these two lines near the top of `/etc/ufw/before.rules`, just after the existing `-A ufw-before-input -i lo -j ACCEPT` line:

```
-A ufw-before-input -i docker0 -j ACCEPT
-A ufw-before-input -i br-+ -j ACCEPT
```

Then turn the firewall on and reload it:

```bash
sudo ufw enable
sudo ufw reload
```

You do not need to open port 3000 to the public. The dashboard is reached through your domain over HTTPS, and Caddy talks to port 3000 internally.

Your server is now ready. Open `https://example.com` in a browser and you should see the Slasha dashboard with a valid HTTPS certificate.

---

## First login: create the admin account

The same `slasha` binary is also the command-line client you manage everything with. Install it on the Linux machine you work from (your workstation) exactly as in step 4 above, then point it at your server:

```bash
slasha set-url https://example.com
```

Now log in. The very first time anyone logs in, Slasha has no users yet, so it walks you through creating the admin account:

```bash
slasha login
```

Enter an email and password when prompted. That first account becomes the admin. Your login token is saved in your machine's keyring, so you stay logged in. Confirm it worked:

```bash
slasha me
```

Running the CLI on a headless box without a desktop keyring (a plain server, or a CI job) is also fine: instead of `slasha login`, hand the CLI a token through the `SLASHA_TOKEN` environment variable and it skips the keyring entirely.

```bash
export SLASHA_TOKEN=<token>
```

---

## Deploy your first app

Deploying is three things: register your SSH key, push your code, and trigger a build.

### Register your SSH key

This is the key Slasha trusts when you `git push`. Add your public key:

```bash
slasha ssh-keys add --file ~/.ssh/id_ed25519.pub --title "my workstation"
```

### Create the app

```bash
slasha create myapp
```

This prints the git remotes for the app. The one you push to looks like:

```
slasha@example.com:myapp.git
```

### Push and deploy

In your project folder, add the remote and push:

```bash
git remote add slasha slasha@example.com:myapp.git
git push -u slasha main
```

Pushing uploads your code but does not deploy by itself. Trigger a build and release:

```bash
slasha deploy --app myapp
```

You can watch the build as it happens:

```bash
slasha deployments logs --app myapp --follow
```

When it finishes, your app is live at:

```
https://myapp.example.com
```

### How the build decides what to do

- If your repository has a `Dockerfile`, Slasha builds that. Slasha reads the `EXPOSE` line to learn which port your app listens on (it assumes `8080` if there is none).
- If there is no `Dockerfile`, Slasha auto-detects your language and builds the image for you (Node, Python, Go, and so on), defaulting to port `8080`.
- If your repo has a `Procfile`, Slasha runs each process type listed in it (for example a `web` process and a `worker` process).

A quick tip: if your app is not a `Dockerfile` app, make sure it listens on the port Slasha expects (`8080`), or add a `Dockerfile` with the right `EXPOSE`.

---

## Day-to-day app management

All of these are run with the CLI from your machine. Add `--app myapp`, or run them from a folder linked to the app with `slasha link`.

Set environment variables for an app:

```bash
slasha env set --app myapp DATABASE_URL=... SECRET_KEY=...
slasha env list --app myapp
```

Run more copies of a process (scale):

```bash
slasha scale --app myapp web=3 worker=1
```

Add a database or cache, attached to the app:

```bash
slasha provision --app myapp --kind postgres --name db --version 16
slasha services list --app myapp
```

Connect to a provisioned service from your machine over a secure tunnel:

```bash
slasha proxy --app myapp db --port 5432
```

Add a custom domain to an app (point the domain's DNS at the server first, then):

```bash
slasha domains add --app myapp www.mysite.com
```

Slasha gets an HTTPS certificate for the new domain automatically.

---

## Where your data lives, and backups

Everything Slasha stores is under the `slasha` user's home directory:

| Path                         | What it holds                        |
|------------------------------|--------------------------------------|
| `/home/slasha/.slasha/slasha.db` | The database (users, apps, settings) |
| `/home/slasha/.slasha/repos/`    | Your apps' git repositories          |
| `/home/slasha/.slasha/logs/`     | Build and run logs                   |
| `/home/slasha/.ssh/authorized_keys` | The push keys Slasha manages (do not edit by hand) |

To back up Slasha, stop the service briefly and copy the `.slasha` folder:

```bash
sudo systemctl stop slasha
sudo tar czf slasha-backup.tar.gz -C /home/slasha .slasha
sudo systemctl start slasha
```

Your apps' HTTPS certificates are stored separately by Caddy in a Docker volume and are re-issued automatically if lost, so they do not need backing up.

---

## Updating Slasha

Re-run the install script to get the newest version, then restart the service:

```bash
curl -fsSL https://raw.githubusercontent.com/slashacom/slasha/main/install.sh | sh
sudo systemctl restart slasha
```

Your data, apps, and certificates are untouched by an upgrade.

---

## Uninstalling

```bash
sudo systemctl disable --now slasha
sudo rm /etc/systemd/system/slasha.service /usr/local/bin/slasha
sudo rm -rf /etc/slasha
```

To also remove all data and the running app containers, delete the `slasha` user's home directory and the Slasha-managed containers:

```bash
sudo docker ps -aq --filter label=slasha.managed=true | xargs -r sudo docker rm -f
sudo userdel -r slasha
```

---

## Command reference

Run `slasha <command> --help` for details on any of these.

| Command                          | What it does |
|----------------------------------|--------------|
| `slasha serve`                   | Run the server (used on the host, via the service). |
| `slasha set-url <url>`           | Point the CLI at your server. |
| `slasha login` / `logout`        | Sign in (creates the admin on first use) or sign out. |
| `slasha me`                      | Show who you are logged in as. |
| `slasha status`                  | Check the server is healthy. |
| `slasha create <name>`           | Create a new app. |
| `slasha list`                    | List your apps. |
| `slasha info`                    | Show an app's details and git remotes. |
| `slasha link`                    | Link the current folder to an app (so you can drop `--app`). |
| `slasha delete`                  | Delete an app and everything it owns. |
| `slasha deploy`                  | Build and release the latest pushed code. |
| `slasha deployments ...`         | List, stop, restart, redeploy, delete, or tail logs of deployments. |
| `slasha env ...`                 | List, set, or unset an app's environment variables. |
| `slasha scale web=2 ...`         | Run more or fewer copies of a process type. |
| `slasha provision ...`           | Add a database or cache (Postgres, MySQL, MongoDB, Redis). |
| `slasha services ...`            | Manage attached services and their logs and env vars. |
| `slasha proxy <service>`         | Tunnel a service to a local port over HTTPS. |
| `slasha domains ...`             | Add, list, or remove custom domains. |
| `slasha ssh-keys ...`            | Add, list, or remove the keys you push with. |
| `slasha users ...`               | (Admin) Add, update, list, or remove users. |
| `slasha version`                 | Print version information. |

Add `--diagnostic` to any command to print system and version details useful for bug reports.

---

## Troubleshooting

The dashboard does not load at your domain.
- Check the service is running: `systemctl status slasha`.
- Check DNS actually points at the server: `dig +short example.com` should return your server's IP.
- Make sure ports 80 and 443 are open and not used by another web server (`nginx`, `apache`). Slasha's Caddy proxy needs both.

The HTTPS certificate is not trusted or did not issue.
- Confirm `SLASHA_ENV=production` is set, then restart the service.
- Certificates need port 80 reachable from the internet and DNS pointing at the server. Behind a CDN proxy, switch the records to "DNS only".

`git push` is rejected or asks for a password.
- Make sure you registered your key with `slasha ssh-keys add` and that you are pushing to the `slasha@...` remote, not an HTTPS one.
- The user in the remote must be `slasha` (for example `slasha@example.com:myapp.git`).

Builds fail immediately.
- Check Slasha can use Docker: the `slasha` user must be in the `docker` group (`groups slasha` should list `docker`). If you just added it, restart the service.
- Watch the build output with `slasha deployments logs --app <name> --follow`.

The app deployed but the URL shows an error.
- Make sure your app listens on the expected port (`8080` unless your `Dockerfile` says otherwise with `EXPOSE`).
- Check the app's logs in the dashboard or with the deployments logs command.

---

## Security notes

- The `JWT_SECRET` signs login tokens. Keep `/etc/slasha/slasha.env` readable only by root, and never commit it anywhere.
- People deploy by connecting over SSH as the `slasha` user, but they can only run Slasha's git handler. The keys Slasha writes into `/home/slasha/.ssh/authorized_keys` are restricted to that one command, so an SSH key cannot open a shell on your server.
- Keep your server patched. On Ubuntu, enabling automatic security updates is a good idea: `sudo apt-get install -y unattended-upgrades`.
- Only the admin (your first account) can manage users. Create additional users with `slasha users create`.
