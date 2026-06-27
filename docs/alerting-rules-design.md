# Alerting Rules & Channels — Design

Status: implemented
Date: 2026-06-27

## As built (summary of divergences from the original proposal)

- Action types are `channel` | `webhook` | `execute_program` (the proposal's
  `send_message`/`send_email` collapsed into `channel` — the channel's own kind
  decides Slack vs Telegram). Mail is deferred (no SMTP dependency pulled in);
  it slots in later behind the same `channels` kind + `Action` path.
- Channels ship with `slack` and `telegram` kinds.
- Events are metric *streams* with dotted names (`server.cpu`, `app.memory`,
  `domain.cert_days`, `domain.dns_problem`, …). Collectors emit the raw reading;
  the rule's `params` threshold (`{"gt":N}` / `{"lt":N}`) decides firing. This is
  cleaner than baking the threshold into the event name.
- v2 collectors are included: per-app metric events (`app.cpu`, `app.memory`,
  targeted `app:<id>` / any `app`) and a scheduled domain-health collector
  (`crates/slasha-server/src/metrics/domain.rs`, 15-min cadence) emitting
  `domain.cert_days` and `domain.dns_problem`.
- Cooldown is persisted per-rule (`alert_rules.last_fired_at`), surviving restarts.
- `server_settings` was fully retired; its data migrates into `channels` + rules.

Key files: `crates/slasha-server/src/alerting/{mod,actions}.rs` (dispatcher +
delivery), `crates/slasha-server/src/metrics/{server,app,domain}.rs` (emitters),
`crates/slasha-db/src/{models,repos}/{channel,alert_rule}.rs`,
`crates/slasha-server/src/routing/api/{channels,alert_rules}.rs`,
`web/app/components/alerting/*`, `web/app/routes/_user.settings.alerts.tsx`.

## Problem

Alerting today is hardwired to one channel and three events:

- `slack_webhook_url` is a literal column on `server_settings`
  (`crates/slasha-db/src/models/server_settings.rs:16`).
- Slack is the only delivery mechanism, hardcoded in `check_alerts`
  (`crates/slasha-server/src/metrics/server.rs:154-164`).
- The only events are the three host thresholds (CPU / memory / disk), evaluated
  inline in the collector (`server.rs:99-135`).

Adding Telegram or a run-command means a schema migration plus a new branch in the
collector. Alerting on cert expiry or an app OOM means copying the whole pattern into
another module. The collector knows about delivery, which it should not.

## Goals

- Decouple three axes that currently move together:
  - what happened (event),
  - the condition that makes it worth alerting (threshold / match),
  - how it's delivered (action / channel).
- Make adding a new event = one emit site, a new channel = one impl + a config row,
  a new alert = a row. No migration per channel type.
- Keep the rule as the center of gravity (Monit model), not a heavyweight
  integrations subsystem.

## Model

Modeled on Monit: a Rules tab with IF/THEN rules, plus reusable connection configs
(Monit's "Mail servers" / "Jabber servers" tabs) for the actions that carry a secret.
Execute-program is inline in the rule because a shell command has no reusable secret.

### `alert_rules` — the IF/THEN rule

| column          | type        | notes                                                        |
|-----------------|-------------|--------------------------------------------------------------|
| `id`            | text PK     |                                                              |
| `name`          | text        | user label, e.g. "High CPU → Slack"                          |
| `enabled`       | bool        | only enabled rules are evaluated                             |
| `target`        | text        | `server` \| `app:<slug>` \| `domain:<id>` \| `any`           |
| `event`         | text        | enum (see below)                                             |
| `params`        | json        | condition params, e.g. `{"gt":85}` or `{"days":7}`           |
| `cooldown_secs` | integer     | per-rule cooldown; default 900                               |
| `action_type`   | text        | `execute_program` \| `send_message` \| `send_email` \| `webhook` |
| `action_config` | json        | inline action config; for channel-backed actions holds `{"channel_id": "..."}` plus optional message template |
| `created_at`    | datetime    |                                                              |
| `updated_at`    | datetime    |                                                              |

### `channels` — reusable connections (only for secret-bearing actions)

| column   | type     | notes                                          |
|----------|----------|------------------------------------------------|
| `id`     | text PK  |                                                |
| `name`   | text     |                                                |
| `type`   | text     | `slack` \| `telegram` \| `mail`                |
| `config` | json     | validated per-type (see below)                 |
| `created_at` / `updated_at` | datetime | |

Channel `config` shapes (validated in code, stored as one json blob so a new channel
type needs no migration):

- `slack`: `{ "webhook_url": "https://hooks.slack.com/..." }`
- `telegram`: `{ "bot_token": "...", "chat_id": "..." }`
- `mail`: `{ "host": "...", "port": 587, "username": "...", "password": "...", "from": "..." }`

`execute_program` and raw `webhook` actions need no channel — their config is fully
inline in `alert_rules.action_config`.

### Events (enum, in code — not user data)

Initial set:

- `cpu_high` — params `{ "gt": <percent> }`
- `memory_high` — params `{ "gt": <percent> }`
- `disk_high` — params `{ "gt": <percent> }`
- `cert_expiring` — params `{ "days": <n> }`  (requires scheduling domain checks — see Open questions)
- `dns_drift` — domain no longer resolves to expected IPs
- `app_oom` — a managed container hit its memory limit / OOM-killed
- `deploy_failed`

New events are added by defining the enum variant and emitting it; rules referencing
unknown events are ignored.

## Flow

1. Collectors stop knowing about Slack. They emit a typed event:
   `Event { target, kind, value }` (e.g. `value` = current CPU %).
2. A dispatcher loads enabled `alert_rules` matching `(target, event)`, evaluates
   `params` against `value`, and applies per-rule cooldown.
3. For each firing rule it resolves the action:
   - channel-backed (`send_message` / `send_email`) → load `channels.config`,
   - inline (`execute_program` / `webhook`) → use `action_config` directly.
4. Delivery is handled per `action_type` behind a single trait:

```rust
struct AlertPayload { /* target, event, value, rendered message */ }

trait Action {
    async fn fire(&self, payload: &AlertPayload) -> anyhow::Result<()>;
}
```

Impls: `SlackAction`, `TelegramAction`, `MailAction`, `WebhookAction`, `CommandAction`.

## Cooldown

Currently per-kind, in-memory (`AlertState`, `server.rs:167-181`), reset on restart.
Move cooldown to per-rule and persist `last_fired_at` on the rule row (or a small
`alert_rule_state` table) so a server restart doesn't re-spam. This also fixes the
flapping-on-boot issue noted earlier.

## execute_program — trust boundary

Decision: run as-is, admin-gated.

The command runs with the slasha server process's privileges. Rule editing already sits
behind the admin middleware (`crates/slasha-server/src/routing/api/mod.rs:24-30`), so the
trust boundary is explicit: anyone who can edit rules can run commands as the server user.
This is documented, not sandboxed, for v1.

To keep blast radius bounded:

- Run via the existing async command path, never a shell string concatenated with
  untrusted input — pass argv, not `sh -c "<interpolated>"`.
- Inject event context as environment variables (`SLASHA_EVENT`, `SLASHA_TARGET`,
  `SLASHA_VALUE`, …) rather than string-interpolating into the command.
- Enforce a timeout and capture stdout/stderr to the tracing log.

## Migration from current state

1. Add `alert_rules` and `channels` tables.
2. Data migration: if `server_settings.slack_webhook_url` is set, seed one `channels`
   row (type `slack`) + three `alert_rules` rows (cpu/memory/disk → that channel,
   thresholds copied from the existing limit columns).
3. Drop `slack_webhook_url`, `cpu_limit_percent`, `memory_limit_percent`,
   `disk_limit_percent` from `server_settings` once the rules path is live.

## API & UI

- `GET/PUT/POST/DELETE /alert-rules` and `/channels` (admin-gated, same middleware
  as `server_settings`).
- Settings UI: a "Rules" section (IF target/event/threshold THEN action) and a
  "Channels" section (configure + test Slack/Telegram/mail once). Mirrors the Monit
  Rules + Mail/Jabber-servers layout.

## Open questions / follow-ups

- `cert_expiring` and `dns_drift` require domain health to run on a schedule and
  persist results — today `check_domains` is on-demand only. That's a prerequisite
  collector, trackable separately.
- `app_oom` requires the app metrics collector to emit events (it currently only
  stores rows). Small addition once the dispatcher exists.
- Message templates (Monit's "Message template" tab) — deferred; start with a default
  rendered message per event, add templating later.

## Scope for v1

- Tables + models + repos for `alert_rules` and `channels`.
- Dispatcher + `Action` trait with Slack, Telegram, webhook, mail, and command impls.
- Refactor server metrics collector to emit events instead of calling Slack directly.
- Migrate existing Slack/threshold settings into rules + a channel.
- Settings UI for rules and channels.

Deferred: scheduled domain-health collector (unlocks cert/dns events), app-metric
events (`app_oom`), message templating.
