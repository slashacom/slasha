use std::collections::HashMap;

use bollard::{
    Docker,
    container::LogOutput,
    models::VolumeCreateRequest,
    query_parameters::{
        LogsOptionsBuilder, RemoveContainerOptionsBuilder, RemoveVolumeOptions,
        StopContainerOptionsBuilder, WaitContainerOptions,
    },
};
use futures_util::StreamExt;

use crate::{
    docker::{DockerError, DockerResult},
    logs::LogWriter,
};

/// Starts a stopped Docker container on a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
pub async fn start_container(docker_client: &Docker, name: &str) -> DockerResult<()> {
    match docker_client
        .start_container(
            name,
            None::<bollard::query_parameters::StartContainerOptions>,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 304, ..
        }) => Ok(()),
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Stops a running Docker container on a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
/// * `timeout_secs` - Optional stop timeout in seconds.
pub async fn stop_container(
    docker_client: &Docker,
    name: &str,
    timeout_secs: Option<i32>,
) -> DockerResult<()> {
    let timeout = timeout_secs.unwrap_or(10);
    let options = StopContainerOptionsBuilder::new().t(timeout).build();

    match docker_client.stop_container(name, Some(options)).await {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            tracing::warn!(container = %name, "Container not found on node during stop operation");
            Ok(())
        }
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Restarts a Docker container on a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
pub async fn restart_container(docker_client: &Docker, name: &str) -> DockerResult<()> {
    docker_client.restart_container(name, None).await?;
    Ok(())
}

/// Force-removes a Docker container from a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
pub async fn remove_container(docker_client: &Docker, name: &str) -> DockerResult<()> {
    let options = RemoveContainerOptionsBuilder::new().force(true).build();

    match docker_client.remove_container(name, Some(options)).await {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            tracing::warn!(container = %name, "Container not found on node during remove operation");
            Ok(())
        }
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Waits for a container to stop and returns its exit code.
///
/// Bollard reports a non-zero exit as an error rather than a status, so a
/// caller that propagated it with `?` would skip whatever cleanup follows and
/// never learn the code. Here a non-zero exit is an ordinary result.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Target container name string.
///
/// # Returns
///
/// A [`DockerResult`] containing the container's exit code.
pub async fn wait_for_exit(docker_client: &Docker, name: &str) -> DockerResult<i64> {
    let next = docker_client
        .wait_container(
            name,
            Some(WaitContainerOptions {
                condition: "not-running".to_string(),
            }),
        )
        .next()
        .await;

    exit_code_from_wait(next)
}

fn exit_code_from_wait(
    next: Option<Result<bollard::models::ContainerWaitResponse, bollard::errors::Error>>,
) -> DockerResult<i64> {
    match next {
        Some(Ok(res)) => Ok(res.status_code),
        Some(Err(bollard::errors::Error::DockerContainerWaitError { code, .. })) => Ok(code),
        Some(Err(e)) => Err(DockerError::from(e)),
        None => Err(DockerError::Other(anyhow::anyhow!(
            "container wait stream ended before the container stopped"
        ))),
    }
}

/// Creates a named Docker volume on a node with optional labels.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Volume name string.
/// * `labels` - Optional label key-value map.
pub async fn create_volume(
    docker_client: &Docker,
    name: &str,
    labels: Option<HashMap<String, String>>,
) -> DockerResult<()> {
    docker_client
        .create_volume(VolumeCreateRequest {
            name: Some(name.to_string()),
            labels,
            ..Default::default()
        })
        .await?;
    Ok(())
}

/// Removes a Docker persistent data volume from a node.
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `name` - Volume name string.
pub async fn remove_volume(docker_client: &Docker, name: &str) -> DockerResult<()> {
    match docker_client
        .remove_volume(name, None::<RemoveVolumeOptions>)
        .await
    {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            tracing::warn!(volume = %name, "Volume not found on node during remove operation");
            Ok(())
        }
        Err(e) => Err(DockerError::from(e)),
    }
}

/// Spawns a background task that streams stdout/stderr container logs from Docker to the [`LogWriter`].
///
/// # Arguments
///
/// * `docker_client` - Docker API client ([`Docker`]).
/// * `log_writer` - Contextual log writer ([`LogWriter`]).
/// * `container` - Container name string.
///
/// # Returns
///
/// A [`tokio::task::JoinHandle`] for the spawned background task.
pub fn stream_container_logs(
    docker_client: Docker,
    log_writer: LogWriter,
    container: String,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move {
        let opts = LogsOptionsBuilder::new()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .build();

        let mut log_stream = docker_client.logs(&container, Some(opts));
        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();

        while let Some(item) = log_stream.next().await {
            match item {
                Ok(LogOutput::StdOut { message } | LogOutput::Console { message }) => {
                    let chunk = String::from_utf8_lossy(&message);
                    flush_lines(&mut stdout_buf, &chunk, |line| log_writer.stdout(line));
                }
                Ok(LogOutput::StdErr { message }) => {
                    let chunk = String::from_utf8_lossy(&message);
                    flush_lines(&mut stderr_buf, &chunk, |line| log_writer.stderr(line));
                }
                Ok(LogOutput::StdIn { .. }) => {}
                Err(err) => {
                    log_writer.stderr(format!("Container log stream error: {err}"));
                    break;
                }
            }
        }

        if !stdout_buf.is_empty() {
            log_writer.stdout(stdout_buf);
        }

        if !stderr_buf.is_empty() {
            log_writer.stderr(stderr_buf);
        }

        Ok(())
    })
}

fn flush_lines(buffer: &mut String, chunk: &str, mut emit: impl FnMut(&str)) {
    buffer.push_str(chunk);

    while let Some(pos) = buffer.find('\n') {
        let line = buffer[..pos].trim_end_matches('\r');
        emit(line);
        buffer.drain(..=pos);
    }
}

#[cfg(test)]
mod tests {
    use bollard::{errors::Error, models::ContainerWaitResponse};

    use super::exit_code_from_wait;

    #[test]
    fn zero_exit_is_a_status() {
        let res = ContainerWaitResponse {
            status_code: 0,
            error: None,
        };
        assert_eq!(exit_code_from_wait(Some(Ok(res))).unwrap(), 0);
    }

    #[test]
    fn non_zero_exit_is_a_status_not_an_error() {
        let err = Error::DockerContainerWaitError {
            error: String::new(),
            code: 1,
        };
        assert_eq!(exit_code_from_wait(Some(Err(err))).unwrap(), 1);
    }

    #[test]
    fn other_errors_and_a_closed_stream_fail() {
        let err = Error::DockerResponseServerError {
            status_code: 404,
            message: "no such container".to_string(),
        };
        assert!(exit_code_from_wait(Some(Err(err))).is_err());
        assert!(exit_code_from_wait(None).is_err());
    }
}
