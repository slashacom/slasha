use std::{path::Path, pin::Pin};

use bollard::{
    Docker,
    container::LogOutput,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
};
use futures_util::{Stream, StreamExt, stream::BoxStream};
use tokio::{
    fs,
    io::{self, AsyncWrite, AsyncWriteExt},
};

use crate::docker::{DockerError, DockerResult};

/// Encapsulates an active Docker exec session attached to standard input and output streams.
pub struct AttachedExec {
    pub docker: Docker,
    pub exec_id: String,
    pub input: Pin<Box<dyn AsyncWrite + Send>>,
    pub output: BoxStream<'static, Result<LogOutput, bollard::errors::Error>>,
}

impl AttachedExec {
    /// Consumes the execution output stream, collecting standard error text into a string.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing accumulated standard error text.
    pub async fn drain_stderr(&mut self) -> DockerResult<String> {
        let mut stderr = String::new();
        while let Some(chunk) = self.output.next().await {
            match chunk {
                Ok(LogOutput::StdErr { message }) => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }
        Ok(stderr)
    }

    /// Inspects the process exit status via the associated Docker client, consuming the exec session.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the optional exit code (`Option<i64>`).
    pub async fn inspect_exit_code(self) -> DockerResult<Option<i64>> {
        let AttachedExec {
            docker, exec_id, ..
        } = self;
        let inspect = docker.inspect_exec(&exec_id).await?;
        Ok(inspect.exit_code)
    }

    /// Streams standard output directly to a file on disk while collecting standard error text.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Local filesystem destination path ([`Path`]).
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the file size in bytes (`i64`) and accumulated standard error text.
    pub async fn stream_stdout_to_file(&mut self, file_path: &Path) -> DockerResult<(i64, String)> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(file_path).await?;
        let mut stderr = String::new();

        while let Some(chunk) = self.output.next().await {
            match chunk {
                Ok(LogOutput::StdOut { message }) => {
                    file.write_all(&message).await?;
                }
                Ok(LogOutput::StdErr { message }) => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }

        file.flush().await?;
        let size = file.metadata().await?.len() as i64;

        Ok((size, stderr))
    }

    /// Pipes an asynchronous byte stream into standard input while draining standard error text.
    ///
    /// # Arguments
    ///
    /// * `stream` - Stream yielding byte chunks.
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing accumulated standard error text.
    pub async fn pipe_stream_and_drain_stderr<S>(&mut self, mut stream: S) -> DockerResult<String>
    where
        S: Stream<Item = Result<bytes::Bytes, io::Error>> + Unpin,
    {
        let input = &mut self.input;
        let output = &mut self.output;

        let write_task = async {
            while let Some(chunk) = stream.next().await {
                input.write_all(&chunk?).await?;
            }
            let _ = input.shutdown().await;
            Ok::<(), io::Error>(())
        };

        let read_task = async {
            let mut stderr = String::new();
            while let Some(chunk) = output.next().await {
                match chunk {
                    Ok(LogOutput::StdErr { message }) => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    Err(e) => return Err(DockerError::from(e)),
                    _ => {}
                }
            }
            Ok::<String, DockerError>(stderr)
        };

        let (write_res, read_res) = tokio::join!(write_task, read_task);
        write_res?;
        read_res
    }
}

/// Creates and starts an attached exec process inside a Docker container.
///
/// # Arguments
///
/// * `docker` - Docker API client ([`Docker`]).
/// * `container` - Name of the target container.
/// * `cmd` - Command argument vector to execute.
/// * `env` - Environment variable vector for command execution.
/// * `attach_stdin` - Whether standard input should be attached (`bool`).
///
/// # Returns
///
/// A [`DockerResult`] containing the running [`AttachedExec`].
pub async fn start_container_exec(
    docker: &Docker,
    container: &str,
    cmd: Vec<String>,
    env: Vec<String>,
    attach_stdin: bool,
) -> DockerResult<AttachedExec> {
    let exec = docker
        .create_exec(
            container,
            CreateExecOptions {
                attach_stdin: Some(attach_stdin),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                cmd: Some(cmd),
                env: if env.is_empty() { None } else { Some(env) },
                ..Default::default()
            },
        )
        .await?;

    match docker
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                ..Default::default()
            }),
        )
        .await?
    {
        StartExecResults::Attached { input, output } => Ok(AttachedExec {
            docker: docker.clone(),
            exec_id: exec.id,
            input,
            output,
        }),
        StartExecResults::Detached => Err(DockerError::ServiceBackupFailed(
            1,
            "exec started detached".into(),
        )),
    }
}
