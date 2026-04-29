# Security model

slasha is a **single-tenant, self-hosted PaaS**. The trust model assumes the operator running slasha owns the host and is the only person deploying apps to it. This document spells out the security boundaries, what slasha does protect, what it does *not*, and how to deploy it more or less defensively.

## Threat model in one line

> Anyone who logs into slasha can deploy a container on the host. Containers run with access to the docker socket, so **slasha login = root on the host**. Treat the slasha admin password the way you'd treat the host's root SSH key.

## What slasha protects against

- **Network-level eavesdropping** — Caddy auto-issues real Let's Encrypt certs in production (`SLASHA_ENV=production`); HSTS is enabled in production responses.
- **Password leaks at rest** — passwords are stored as Argon2id hashes (default OWASP-recommended params).
- **Token forgery** — JWT auth uses HS256 with a 256-bit secret; the verifier pins the algorithm so `alg=none` and key-confusion attacks don't apply.
- **User enumeration** — `/login` returns the same error message for unknown email and bad password.
- **First-boot squatting** — `/signup` is locked permanently after the first admin is created. `/signup` and `/login` are rate-limited per source IP (3/min and 10/min respectively).

## What slasha does NOT currently protect against

These are real gaps. Treat them as load-bearing constraints when planning your deploy.

- **No MFA / WebAuthn.** Login is password-only. The login form is the gate to everything; if your password is weak, an attacker who breaks it owns the host. *Choose a strong password.* MFA is a roadmap item.
- **JWT revocation.** Tokens are valid for 30 days from issue. There is no revocation list, so a leaked token stays valid until expiry — even after a password change.
- **Audit log.** Deployments, logins, and admin changes are not separately logged for forensic review.
- **Container isolation.** User-app containers spawned by slasha do not currently apply capability drops, seccomp profiles, user namespaces, or memory/CPU limits. For a self-hosted single-tenant install where you trust your own apps, this is fine. *Do not run untrusted third-party apps.*
- **Docker socket exposure.** The slasha container has `/var/run/docker.sock` mounted read-write. Any RCE in slasha-server escalates to root on the host. The high-level mitigation is "don't expose the slasha login to the public internet if you don't need to" — see `SLASHA_PRIVATE_MODE` below.

## Deployment recommendations

Pick one of the three based on your tolerance for public-facing auth surface.

### 1. Public dashboard (default)

DNS for both `your.domain` and `*.your.domain` points at the host. Caddy serves the dashboard publicly with a real cert; `/login` is rate-limited; you log in with your password.

This is the simplest setup and what the project README walks you through. Fine for solo operators with strong passwords. **Not recommended for production until MFA lands** if your password might be guessable.

### 2. Private dashboard, public apps

Set `SLASHA_PRIVATE_MODE=true`. The apex domain is omitted from Caddy's routes — the dashboard is no longer reachable from the internet. App subdomains (`<slug>.your.domain`) stay public so users hitting your deployed apps still work.

To use the dashboard yourself:
```sh
ssh -L 3000:127.0.0.1:3000 root@your-host
# then open http://localhost:3000 in your browser
```

This removes the auth surface from the internet entirely. Recommended for any host you don't actively administrate from a phone.

### 3. Behind a VPN

Bind your host's public ports only to your VPN (Tailscale, WireGuard, etc.). All slasha traffic — apps, dashboard, git push — flows over the VPN. Maximally restrictive; only suitable when your app users are also on your VPN.

## Hardening the host itself

The deployment scaffold ships with these enabled:

- `ufw` deny-all-incoming except `22` (host ssh), `80`/`443` (Caddy), `2222` (slasha git push).
- `fail2ban` with jails for both host sshd and the container's git-push sshd.
- sshd host keys persist in a Docker volume across rebuilds (clients won't see "host key changed" warnings on every deploy).
- Caddy adds `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`, and (in production) `Strict-Transport-Security` to all responses.

## Rotating the JWT secret

Rotating `JWT_SECRET` invalidates every existing session immediately. Useful if you suspect a token leak.

```sh
# in your slasha-infra repo (or wherever your prod env lives)
new=$(openssl rand -hex 32)
sed -i.bak "s/^JWT_SECRET=.*/JWT_SECRET=$new/" .env.production && rm .env.production.bak
./deploy.sh --no-build
```

## Reporting security issues

Please email **kamranahmed.se@gmail.com** with details. Don't open public issues for security-sensitive reports.
