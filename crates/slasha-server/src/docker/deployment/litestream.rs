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
/// and the absolute paths of the bundled binaries within it.
pub const CONTAINER_MOUNT_DIR: &str = "/slasha";
pub const CONTAINER_BINARY_PATH: &str = "/slasha/litestream";
/// Statically-linked sqlite3 CLI, used to put the database in WAL mode (required
/// by litestream) without the user having to configure it in their app.
pub const CONTAINER_SQLITE_PATH: &str = "/slasha/sqlite3";
/// MinIO client (static), used to probe the replica's reachability and freshness
/// for the health signal.
pub const CONTAINER_MC_PATH: &str = "/slasha/mc";

/// Shared, read-only docker volume holding the litestream and sqlite3 binaries.
/// A single volume is reused across all apps; it is populated once (see
/// [`ensure_litestream_volume`]).
pub const LITESTREAM_VOLUME: &str = "slasha-litestream";

const LITESTREAM_VERSION: &str = "v0.3.13";
const SQLITE_AMALGAMATION_URL: &str =
    "https://sqlite.org/2024/sqlite-amalgamation-3460100.zip";
const SQLITE_AMALGAMATION_DIR: &str = "sqlite-amalgamation-3460100";
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
    let sqlite = CONTAINER_SQLITE_PATH;
    let cfg = CONFIG_PATH;
    let db = shell_single_quote(&backup.db_path);
    let yaml = config_yaml(backup);
    let exec_cmd = shell_single_quote(original_cmd);

    let restore = if restore_pending {
        // Forced point-in-time restore. Never touch the live database until a
        // verified replacement exists: restore to a temp path, integrity-check
        // it, then swap atomically. Any failure keeps the existing database.
        format!(
            "TMP={db}.slasha-restore\n\
             rm -f \"$TMP\" \"$TMP\"-wal \"$TMP\"-shm 2>/dev/null || true\n\
             if {bin} restore -o \"$TMP\" -if-replica-exists -config {cfg} {db}; then\n\
             if [ -f \"$TMP\" ] && {sqlite} \"$TMP\" 'PRAGMA integrity_check;' 2>/dev/null | head -1 | grep -qx ok; then\n\
             mv -f \"$TMP\" {db}\n\
             rm -f {db}-wal {db}-shm 2>/dev/null || true\n\
             echo 'slasha: restored database from replica'\n\
             else\n\
             rm -f \"$TMP\" 2>/dev/null || true\n\
             echo 'slasha: no valid replica to restore; keeping existing database' >&2\n\
             fi\n\
             else\n\
             rm -f \"$TMP\" 2>/dev/null || true\n\
             echo 'slasha: restore failed; keeping existing database' >&2\n\
             fi"
        )
    } else {
        // Normal boot: only restore when the database is missing. If a restore
        // genuinely errors (a replica likely exists but is unreachable), abort
        // rather than start empty and replicate an empty database over a good
        // backup — the container restarts and retries.
        format!(
            "if [ ! -f {db} ]; then\n\
             if {bin} restore -if-replica-exists -config {cfg} {db}; then :; else\n\
             echo 'slasha: database missing and restore failed; aborting to avoid replicating an empty database' >&2\n\
             exit 1\n\
             fi\n\
             fi"
        )
    };

    // Litestream only replicates WAL-mode databases. Put the database in WAL
    // mode if it already exists (after restore, or once the app has created it
    // on a prior boot) so the user doesn't have to set it in their app. This is
    // a no-op when the database is already WAL.
    let ensure_wal =
        format!("if [ -f {db} ]; then {sqlite} {db} 'PRAGMA journal_mode=WAL;' >/dev/null 2>&1 || true; fi");

    let command = format!(
        "set -e\nmkdir -p \"$(dirname {db})\"\ncat > {cfg} <<'SLASHA_LITESTREAM_EOF'\n{yaml}SLASHA_LITESTREAM_EOF\n{restore}\n{ensure_wal}\nexec {bin} replicate -config {cfg} -exec {exec_cmd}"
    );

    let mut env = HashMap::new();
    env.insert(ACCESS_KEY_ENV.to_string(), backup.access_key_id.clone());
    env.insert(SECRET_KEY_ENV.to_string(), backup.secret_access_key.clone());

    LitestreamPlan { command, env }
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
    let sqlite_url = SQLITE_AMALGAMATION_URL;
    let sqlite_dir = SQLITE_AMALGAMATION_DIR;
    format!(
        "set -e\n\
         if [ -f /dst/litestream ] && [ -f /dst/sqlite3 ] && [ -f /dst/mc ]; then exit 0; fi\n\
         apk add -q --no-cache wget tar unzip gcc musl-dev >/dev/null 2>&1\n\
         ARCH=$(uname -m)\n\
         case \"$ARCH\" in x86_64) A=amd64;; aarch64) A=arm64;; *) echo \"unsupported arch $ARCH\" >&2; exit 1;; esac\n\
         wget -qO /tmp/ls.tar.gz \"https://github.com/benbjohnson/litestream/releases/download/{ver}/litestream-{ver}-linux-${{A}}.tar.gz\"\n\
         tar -xzf /tmp/ls.tar.gz -C /dst litestream\n\
         chmod +x /dst/litestream\n\
         wget -qO /dst/mc \"https://dl.min.io/client/mc/release/linux-${{A}}/mc\"\n\
         chmod +x /dst/mc\n\
         wget -qO /tmp/sqlite.zip \"{sqlite_url}\"\n\
         unzip -q /tmp/sqlite.zip -d /tmp\n\
         gcc -Os -static -DSQLITE_THREADSAFE=0 -DSQLITE_OMIT_LOAD_EXTENSION -o /dst/sqlite3 /tmp/{sqlite_dir}/shell.c /tmp/{sqlite_dir}/sqlite3.c -lm\n\
         chmod +x /dst/sqlite3\n"
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

/// Result of checking the object-storage replica directly.
pub struct ReplicaProbe {
    /// Whether the bucket could be listed with the configured credentials —
    /// this is the real health signal (catches bad creds, wrong bucket,
    /// unreachable endpoint), unlike "the process is running".
    pub reachable: bool,
    /// A short error excerpt when not reachable.
    pub error: Option<String>,
    /// Time of the most recently written replica object, when any exist.
    pub last_synced_at: Option<chrono::NaiveDateTime>,
}

const PROBE_OK_MARKER: &str = "SLASHA_REACHABLE";

/// Probe the replica directly via the bundled `mc` client: list the bucket
/// prefix. A successful list means credentials/bucket/endpoint are valid (the
/// thing the "process is up" signal can't tell you); the newest object's time
/// approximates the last sync.
pub async fn probe_replica(docker: &Docker, backup: &AppBackup) -> anyhow::Result<ReplicaProbe> {
    ensure_litestream_volume(docker).await?;

    let mc = CONTAINER_MC_PATH;
    let endpoint = shell_single_quote(&backup.endpoint);
    let bucket = shell_single_quote(&backup.bucket);
    let prefix = shell_single_quote(&replica_path(backup));
    // Credentials come from the environment; the secret is never in the script.
    let script = format!(
        "OUT=$({mc} alias set slasha {endpoint} \"${ACCESS_KEY_ENV}\" \"${SECRET_KEY_ENV}\" 2>&1 \
         && {mc} ls --recursive --json slasha/{bucket}/{prefix}/ 2>&1)\n\
         RC=$?\n\
         if [ $RC -eq 0 ]; then printf '{PROBE_OK_MARKER}\\n%s\\n' \"$OUT\"; else printf '%s\\n' \"$OUT\"; fi"
    );
    let env = vec![
        format!("{ACCESS_KEY_ENV}={}", backup.access_key_id),
        format!("{SECRET_KEY_ENV}={}", backup.secret_access_key),
    ];

    let output = run_capture_container(docker, &script, env).await?;

    if let Some(rest) = output.strip_prefix(PROBE_OK_MARKER).or_else(|| {
        output
            .find(PROBE_OK_MARKER)
            .map(|i| &output[i + PROBE_OK_MARKER.len()..])
    }) {
        Ok(ReplicaProbe {
            reachable: true,
            error: None,
            last_synced_at: parse_last_modified(rest),
        })
    } else {
        let error = output.trim();
        Ok(ReplicaProbe {
            reachable: false,
            error: (!error.is_empty()).then(|| error.chars().take(300).collect()),
            last_synced_at: None,
        })
    }
}

/// Extract the newest `"lastModified":"<rfc3339>"` value from `mc ls --json` output.
pub fn parse_last_modified(output: &str) -> Option<chrono::NaiveDateTime> {
    output
        .split("\"lastModified\":\"")
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .filter_map(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.naive_utc())
        .max()
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
            last_checked_at: None,
            last_check_ok: None,
            last_check_error: None,
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
    fn normal_boot_restores_only_if_missing_and_aborts_on_error() {
        let plan = plan(&backup(), "node server.js", false);
        // Only restores when the DB is missing, and never deletes anything.
        assert!(plan.command.contains("if [ ! -f '/data/app.db' ]"));
        assert!(plan.command.contains("restore -if-replica-exists"));
        assert!(!plan.command.contains("rm -f"));
        // Aborts instead of starting empty when a restore genuinely fails.
        assert!(plan.command.contains("aborting to avoid replicating an empty database"));
        assert!(plan.command.contains("exit 1"));
        assert!(
            plan.command
                .contains(&format!("exec {CONTAINER_BINARY_PATH} replicate"))
        );
        assert!(plan.command.contains("-exec 'node server.js'"));
        assert!(
            plan.command
                .contains(&format!("{CONTAINER_SQLITE_PATH} '/data/app.db' 'PRAGMA journal_mode=WAL;'"))
        );
    }

    #[test]
    fn forced_restore_never_destroys_live_db_before_verified_swap() {
        let plan = plan(&backup(), "node server.js", true);
        // Restores to a temp path, integrity-checks, then swaps — never rm's the
        // live DB up front.
        assert!(plan.command.contains("'/data/app.db'.slasha-restore"));
        assert!(plan.command.contains("restore -o \"$TMP\" -if-replica-exists"));
        assert!(plan.command.contains("PRAGMA integrity_check;"));
        assert!(plan.command.contains("mv -f \"$TMP\" '/data/app.db'"));
        assert!(plan.command.contains("keeping existing database"));
        // The dangerous original pattern (delete live DB first) must be gone.
        assert!(!plan.command.contains("rm -f '/data/app.db'*"));
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
    fn generated_command_is_valid_posix_shell() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        for pending in [false, true] {
            let plan = plan(&backup(), "node server.js", pending);
            let mut child = Command::new("sh")
                .arg("-n")
                .stdin(Stdio::piped())
                .spawn()
                .expect("spawn sh");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(plan.command.as_bytes())
                .unwrap();
            assert!(
                child.wait().unwrap().success(),
                "generated shell is not valid (restore_pending={pending}):\n{}",
                plan.command
            );
        }
    }

    #[test]
    fn parses_newest_last_modified_from_mc_json() {
        // Real `mc ls --recursive --json` lines.
        let output = "{\"status\":\"success\",\"type\":\"file\",\"lastModified\":\"2026-06-21T00:26:59.357Z\",\"size\":368}\n\
             {\"status\":\"success\",\"type\":\"file\",\"lastModified\":\"2026-06-21T00:27:03.341Z\",\"size\":4120}\n";
        let parsed = parse_last_modified(output).unwrap();
        assert_eq!(parsed.to_string(), "2026-06-21 00:27:03.341");
    }

    #[test]
    fn parses_no_last_modified_when_empty() {
        assert!(parse_last_modified("").is_none());
        assert!(parse_last_modified("{\"status\":\"success\"}\n").is_none());
    }

    #[tokio::test]
    #[ignore = "requires a docker daemon with network access"]
    async fn ensure_volume_populates_binary() {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let volume = ensure_litestream_volume(&docker).await.unwrap();
        assert_eq!(volume, LITESTREAM_VOLUME);
    }
}
