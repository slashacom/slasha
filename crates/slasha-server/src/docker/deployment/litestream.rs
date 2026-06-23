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

pub const CONTAINER_MOUNT_DIR: &str = "/slasha";
pub const CONTAINER_BINARY_PATH: &str = "/slasha/litestream";
pub const CONTAINER_SQLITE_PATH: &str = "/slasha/sqlite3";
pub const CONTAINER_MC_PATH: &str = "/slasha/mc";

pub const LITESTREAM_VOLUME: &str = "slasha-litestream";

const LITESTREAM_VERSION: &str = "v0.3.13";
const SQLITE_AMALGAMATION_URL: &str = "https://sqlite.org/2024/sqlite-amalgamation-3460100.zip";
const SQLITE_AMALGAMATION_DIR: &str = "sqlite-amalgamation-3460100";
const HELPER_IMAGE: &str = "alpine:3.20";

const ACCESS_KEY_ENV: &str = "LITESTREAM_ACCESS_KEY_ID";
const SECRET_KEY_ENV: &str = "LITESTREAM_SECRET_ACCESS_KEY";

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

fn config_yaml(backup: &AppBackup) -> String {
    let db_path = &backup.db_path;
    let bucket = &backup.bucket;
    let endpoint = &backup.endpoint;
    let path = replica_path(backup);
    format!(
        "dbs:\n  - path: {db_path}\n    replicas:\n      - type: s3\n        bucket: {bucket}\n        path: {path}\n        endpoint: {endpoint}\n        region: auto\n        force-path-style: true\n"
    )
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn plan(backup: &AppBackup, original_cmd: &str, restore_pending: bool) -> LitestreamPlan {
    let bin = CONTAINER_BINARY_PATH;
    let sqlite = CONTAINER_SQLITE_PATH;
    let cfg = CONFIG_PATH;
    let db = shell_single_quote(&backup.db_path);
    let yaml = config_yaml(backup);
    let exec_cmd = shell_single_quote(original_cmd);

    let restore = if restore_pending {
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
        format!(
            "if [ ! -f {db} ]; then\n\
             if {bin} restore -if-replica-exists -config {cfg} {db}; then :; else\n\
             echo 'slasha: database missing and restore failed; aborting to avoid replicating an empty database' >&2\n\
             exit 1\n\
             fi\n\
             fi"
        )
    };

    let ensure_wal = format!(
        "if [ -f {db} ]; then {sqlite} {db} 'PRAGMA journal_mode=WAL;' >/dev/null 2>&1 || true; fi"
    );

    let command = format!(
        "set -e\nmkdir -p \"$(dirname {db})\"\ncat > {cfg} <<'SLASHA_LITESTREAM_EOF'\n{yaml}SLASHA_LITESTREAM_EOF\n{restore}\n{ensure_wal}\nexec {bin} replicate -config {cfg} -exec {exec_cmd}"
    );

    let mut env = HashMap::new();
    env.insert(ACCESS_KEY_ENV.to_string(), backup.access_key_id.clone());
    env.insert(SECRET_KEY_ENV.to_string(), backup.secret_access_key.clone());

    LitestreamPlan { command, env }
}

pub fn binary_mount() -> Mount {
    Mount {
        typ: Some(MountTypeEnum::VOLUME),
        source: Some(LITESTREAM_VOLUME.to_string()),
        target: Some(CONTAINER_MOUNT_DIR.to_string()),
        read_only: Some(true),
        ..Default::default()
    }
}

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
        .start_container(
            container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
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

pub struct ReplicaProbe {
    pub reachable: bool,
    pub error: Option<String>,
    pub last_synced_at: Option<chrono::NaiveDateTime>,
}

const PROBE_OK_MARKER: &str = "SLASHA_REACHABLE";

pub async fn probe_replica(docker: &Docker, backup: &AppBackup) -> anyhow::Result<ReplicaProbe> {
    ensure_litestream_volume(docker).await?;

    let mc = CONTAINER_MC_PATH;
    let endpoint = shell_single_quote(&backup.endpoint);
    let bucket = shell_single_quote(&backup.bucket);
    let prefix = shell_single_quote(&replica_path(backup));

    // credentials come from the environment; the secret is never in the script.
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

pub fn parse_last_modified(output: &str) -> Option<chrono::NaiveDateTime> {
    output
        .split("\"lastModified\":\"")
        .skip(1)
        .filter_map(|chunk| chunk.split('"').next())
        .filter_map(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.naive_utc())
        .max()
}

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

async fn capture_container_output(docker: &Docker, container_name: &str) -> anyhow::Result<String> {
    docker
        .start_container(
            container_name,
            Some(StartContainerOptionsBuilder::new().build()),
        )
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
