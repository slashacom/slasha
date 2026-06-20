//! Wraps an app's start command with [Litestream](https://litestream.io) so a
//! SQLite database is restored on boot and continuously replicated to an
//! S3-compatible bucket (e.g. Cloudflare R2) for the lifetime of the process.
//!
//! Litestream must be a *single writer* per database, so this wrap is only ever
//! applied to one container (the primary web instance). The app's other
//! processes read and write the SQLite file normally; Litestream observes the
//! WAL regardless of which process produced it.

use std::collections::HashMap;

use bollard::{
    Docker,
    models::{HostConfig, Mount, MountTypeEnum, VolumeCreateRequest},
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, LogsOptionsBuilder,
        RemoveContainerOptionsBuilder, StartContainerOptionsBuilder, WaitContainerOptions,
    },
};
use futures_util::StreamExt;
use slasha_db::app_backup::AppBackup;
use uuid::Uuid;

const CONFIG_PATH: &str = "/etc/litestream.yml";

/// Directory the shared litestream volume is mounted at inside app containers,
/// and the absolute path of the binary within it.
pub const CONTAINER_MOUNT_DIR: &str = "/slasha";
pub const CONTAINER_BINARY_PATH: &str = "/slasha/litestream";

/// Shared, read-only docker volume holding the litestream binary. A single
/// volume is reused across all apps; it is populated once from the pinned
/// GitHub release (see [`ensure_litestream_volume`]).
pub const LITESTREAM_VOLUME: &str = "slasha-litestream";

const LITESTREAM_VERSION: &str = "v0.3.13";
const HELPER_IMAGE: &str = "alpine:3.20";

const ACCESS_KEY_ENV: &str = "LITESTREAM_ACCESS_KEY_ID";
const SECRET_KEY_ENV: &str = "LITESTREAM_SECRET_ACCESS_KEY";

/// A litestream-wrapped start command plus the extra (secret) environment it
/// needs. The secret access key is passed via the environment, never embedded
/// in the command string, so it does not leak into the process table.
pub struct LitestreamPlan {
    pub command: String,
    pub env: HashMap<String, String>,
}

fn replica_path(backup: &AppBackup) -> String {
    match &backup.path_prefix {
        Some(prefix) if !prefix.is_empty() => prefix.clone(),
        _ => "litestream".to_string(),
    }
}

/// Litestream config. Credentials are read from the environment by Litestream
/// itself, so they are deliberately absent here.
fn config_yaml(backup: &AppBackup) -> String {
    let db_path = &backup.db_path;
    let bucket = &backup.bucket;
    let endpoint = &backup.endpoint;
    let path = replica_path(backup);
    format!(
        "dbs:\n  - path: {db_path}\n    replicas:\n      - type: s3\n        bucket: {bucket}\n        path: {path}\n        endpoint: {endpoint}\n        region: auto\n        force-path-style: true\n"
    )
}

/// Wrap a value in single quotes for safe inclusion in a `sh -c` string.
fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Build the litestream-wrapped command for `original_cmd`.
///
/// When `restore_pending` is set, the local database is discarded and restored
/// from the replica before the app starts (a point-in-time recovery). Otherwise
/// the database is only restored if it is missing — the normal boot path.
pub fn plan(backup: &AppBackup, original_cmd: &str, restore_pending: bool) -> LitestreamPlan {
    let bin = CONTAINER_BINARY_PATH;
    let cfg = CONFIG_PATH;
    let db = shell_single_quote(&backup.db_path);
    let yaml = config_yaml(backup);
    let exec_cmd = shell_single_quote(original_cmd);

    let restore = if restore_pending {
        format!(
            "rm -f {db}* 2>/dev/null || true\n{bin} restore -if-replica-exists -config {cfg} {db} || true"
        )
    } else {
        format!("{bin} restore -if-db-not-exists -if-replica-exists -config {cfg} {db} || true")
    };

    let command = format!(
        "set -e\nmkdir -p \"$(dirname {db})\"\ncat > {cfg} <<'SLASHA_LITESTREAM_EOF'\n{yaml}SLASHA_LITESTREAM_EOF\n{restore}\nexec {bin} replicate -config {cfg} -exec {exec_cmd}"
    );

    let mut env = HashMap::new();
    env.insert(ACCESS_KEY_ENV.to_string(), backup.access_key_id.clone());
    env.insert(SECRET_KEY_ENV.to_string(), backup.secret_access_key.clone());

    LitestreamPlan { command, env }
}

/// Parse the most recent replicated timestamp from `litestream generations`
/// output. The `end` column holds the time of the last replicated WAL; we take
/// the latest RFC3339 timestamp anywhere in the output to stay tolerant of
/// formatting changes across litestream versions.
pub fn parse_last_synced(output: &str) -> Option<chrono::NaiveDateTime> {
    output
        .split_whitespace()
        .filter_map(|token| chrono::DateTime::parse_from_rfc3339(token).ok())
        .map(|dt| dt.naive_utc())
        .max()
}

/// Read-only mount of the shared litestream volume for an app container.
pub fn binary_mount() -> Mount {
    Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(LITESTREAM_VOLUME.to_string()),
        target: Some(CONTAINER_MOUNT_DIR.to_string()),
        read_only: Some(true),
        ..Default::default()
    }
}

/// Shell script (run in the helper container) that downloads the pinned
/// litestream release for the host architecture into the shared volume. Idempotent.
fn populate_script() -> String {
    let ver = LITESTREAM_VERSION;
    format!(
        "set -e\n\
         if [ -f /dst/litestream ]; then exit 0; fi\n\
         apk add -q --no-cache wget tar >/dev/null 2>&1\n\
         ARCH=$(uname -m)\n\
         case \"$ARCH\" in x86_64) A=amd64;; aarch64) A=arm64;; *) echo \"unsupported arch $ARCH\" >&2; exit 1;; esac\n\
         wget -qO /tmp/ls.tar.gz \"https://github.com/benbjohnson/litestream/releases/download/{ver}/litestream-{ver}-linux-${{A}}.tar.gz\"\n\
         tar -xzf /tmp/ls.tar.gz -C /dst litestream\n\
         chmod +x /dst/litestream\n"
    )
}

/// Ensure the shared litestream volume exists and contains the binary, populating
/// it from the pinned GitHub release via a throwaway alpine container if needed.
/// Idempotent and safe to call on every backup-enabled deploy. Returns the volume
/// name on success.
pub async fn ensure_litestream_volume(docker: &Docker) -> anyhow::Result<String> {
    docker
        .create_volume(VolumeCreateRequest {
            name: Some(LITESTREAM_VOLUME.to_string()),
            ..Default::default()
        })
        .await?;

    let mut pull = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(HELPER_IMAGE.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(result) = pull.next().await {
        result?;
    }

    let container_name = format!("slasha-litestream-setup-{}", Uuid::new_v4());
    docker
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            bollard::models::ContainerCreateBody {
                image: Some(HELPER_IMAGE.to_string()),
                cmd: Some(vec!["sh".to_string(), "-c".to_string(), populate_script()]),
                host_config: Some(HostConfig {
                    mounts: Some(vec![Mount {
                        typ: Some(MountTypeEnum::VOLUME),
                        source: Some(LITESTREAM_VOLUME.to_string()),
                        target: Some("/dst".to_string()),
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await?;

    let result = run_setup_container(docker, &container_name).await;

    let _ = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await;

    result?;
    Ok(LITESTREAM_VOLUME.to_string())
}

async fn run_setup_container(docker: &Docker, container_name: &str) -> anyhow::Result<()> {
    docker
        .start_container(container_name, Some(StartContainerOptionsBuilder::new().build()))
        .await?;

    let wait = docker
        .wait_container(
            container_name,
            Some(WaitContainerOptions {
                condition: "not-running".to_string(),
            }),
        )
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("litestream setup container ended prematurely"))??;

    if wait.status_code != 0 {
        anyhow::bail!(
            "litestream setup container exited with status {}",
            wait.status_code
        );
    }
    Ok(())
}

/// Query the replica for the time of the last replicated WAL by running
/// `litestream generations` in a one-shot container. Best-effort: returns `None`
/// when nothing has been replicated yet or the replica can't be read.
pub async fn probe_last_synced(
    docker: &Docker,
    backup: &AppBackup,
) -> anyhow::Result<Option<chrono::NaiveDateTime>> {
    ensure_litestream_volume(docker).await?;

    let bin = CONTAINER_BINARY_PATH;
    let cfg = CONFIG_PATH;
    let yaml = config_yaml(backup);
    let db = shell_single_quote(&backup.db_path);
    let script = format!(
        "cat > {cfg} <<'SLASHA_LITESTREAM_EOF'\n{yaml}SLASHA_LITESTREAM_EOF\n{bin} generations -config {cfg} {db} 2>/dev/null || true"
    );
    let env = vec![
        format!("{ACCESS_KEY_ENV}={}", backup.access_key_id),
        format!("{SECRET_KEY_ENV}={}", backup.secret_access_key),
    ];

    let output = run_capture_container(docker, &script, env).await?;
    Ok(parse_last_synced(&output))
}

/// Run a short alpine command with the litestream binary mounted and capture its
/// stdout. Used for read-only replica queries.
async fn run_capture_container(
    docker: &Docker,
    script: &str,
    env: Vec<String>,
) -> anyhow::Result<String> {
    let container_name = format!("slasha-litestream-probe-{}", Uuid::new_v4());
    docker
        .create_container(
            Some(CreateContainerOptions {
                name: Some(container_name.clone()),
                ..Default::default()
            }),
            bollard::models::ContainerCreateBody {
                image: Some(HELPER_IMAGE.to_string()),
                cmd: Some(vec!["sh".to_string(), "-c".to_string(), script.to_string()]),
                env: Some(env),
                host_config: Some(HostConfig {
                    mounts: Some(vec![binary_mount()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await?;

    let result = capture_container_output(docker, &container_name).await;

    let _ = docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptionsBuilder::new().force(true).build()),
        )
        .await;

    result
}

async fn capture_container_output(
    docker: &Docker,
    container_name: &str,
) -> anyhow::Result<String> {
    docker
        .start_container(container_name, Some(StartContainerOptionsBuilder::new().build()))
        .await?;

    docker
        .wait_container(
            container_name,
            Some(WaitContainerOptions {
                condition: "not-running".to_string(),
            }),
        )
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("litestream probe container ended prematurely"))??;

    let mut logs = docker.logs(
        container_name,
        Some(LogsOptionsBuilder::new().stdout(true).stderr(false).build()),
    );
    let mut output = String::new();
    while let Some(chunk) = logs.next().await {
        output.push_str(&chunk?.to_string());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn backup() -> AppBackup {
        AppBackup {
            id: "b1".into(),
            app_id: "a1".into(),
            enabled: true,
            db_path: "/data/app.db".into(),
            bucket: "my-app-db".into(),
            endpoint: "https://acct.r2.cloudflarestorage.com".into(),
            path_prefix: Some("prod".into()),
            access_key_id: "AKID".into(),
            secret_access_key: "SECRET".into(),
            restore_pending: false,
            last_synced_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn config_has_replica_settings_but_no_secret() {
        let yaml = config_yaml(&backup());
        assert!(yaml.contains("bucket: my-app-db"));
        assert!(yaml.contains("path: prod"));
        assert!(yaml.contains("endpoint: https://acct.r2.cloudflarestorage.com"));
        assert!(yaml.contains("region: auto"));
        assert!(!yaml.contains("SECRET"));
    }

    #[test]
    fn normal_boot_restores_only_if_missing_and_execs_original() {
        let plan = plan(&backup(), "node server.js", false);
        assert!(plan.command.contains("restore -if-db-not-exists -if-replica-exists"));
        assert!(!plan.command.contains("rm -f"));
        assert!(
            plan.command
                .contains(&format!("exec {CONTAINER_BINARY_PATH} replicate"))
        );
        assert!(plan.command.contains("-exec 'node server.js'"));
    }

    #[test]
    fn restore_pending_discards_local_db_first() {
        let plan = plan(&backup(), "node server.js", true);
        assert!(plan.command.contains("rm -f '/data/app.db'*"));
        assert!(plan.command.contains("restore -if-replica-exists"));
        assert!(!plan.command.contains("-if-db-not-exists"));
    }

    #[test]
    fn secret_travels_via_env_not_command() {
        let plan = plan(&backup(), "node server.js", false);
        assert_eq!(plan.env.get(SECRET_KEY_ENV).map(String::as_str), Some("SECRET"));
        assert_eq!(plan.env.get(ACCESS_KEY_ENV).map(String::as_str), Some("AKID"));
        assert!(!plan.command.contains("SECRET"));
    }

    #[test]
    fn single_quotes_in_original_command_are_escaped() {
        let plan = plan(&backup(), "sh -c 'echo hi'", false);
        assert!(plan.command.contains(r"'\''"));
    }

    #[test]
    fn parses_last_synced_from_generations_output() {
        // Real `litestream generations` output (v0.3.13).
        let output = "name  generation        lag  start                 end\n\
             s3    8caab87c2367a2a8  -1s  2026-06-20T16:30:08Z  2026-06-20T16:30:12Z\n";
        let parsed = parse_last_synced(output).unwrap();
        assert_eq!(parsed.to_string(), "2026-06-20 16:30:12");
    }

    #[test]
    fn parses_none_when_no_generations() {
        assert!(parse_last_synced("no generations available\n").is_none());
        assert!(parse_last_synced("").is_none());
    }

    #[tokio::test]
    #[ignore = "requires a docker daemon with network access"]
    async fn ensure_volume_populates_binary() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let volume = ensure_litestream_volume(&docker).await.unwrap();
        assert_eq!(volume, LITESTREAM_VOLUME);
    }
}
