use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use tokio_util::sync::CancellationToken;

use super::journal::RollbackJournal;
use crate::{
    docker::{DockerError, DockerResult},
    logs::LogWriter,
};

pub struct WorkflowContext<'a> {
    pub name: String,
    pub journal: RollbackJournal,
    pub log: Option<&'a LogWriter>,
}

impl<'a> WorkflowContext<'a> {
    /// Executes a named workflow step with automatic forward logging and undo registration.
    ///
    /// # Arguments
    ///
    /// * `name` - Descriptive step name string.
    /// * `action` - Future executing the primary step logic.
    /// * `undo` - Future executing the rollback undo action on failure.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the action output or error.
    pub async fn step<FutAction, FutUndo, T, E>(
        &self,
        step_name: impl std::fmt::Display,
        action: FutAction,
        undo: FutUndo,
    ) -> Result<T, E>
    where
        FutAction: std::future::Future<Output = Result<T, E>>,
        FutUndo: std::future::Future<Output = ()> + Send + 'static,
    {
        tracing::info!(workflow = %self.name, step = %step_name, "executing workflow step");
        if let Some(log) = self.log {
            log.stdout(format!("{}", step_name));
        }

        let result = action.await;
        if result.is_ok() {
            self.journal.push(step_name.to_string(), undo);
        }

        result
    }
}

pub struct WorkflowRunner<'a> {
    name: String,
    log: Option<&'a LogWriter>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> WorkflowRunner<'a> {
    /// Creates a new [`WorkflowRunner`] for a named workflow.
    ///
    /// # Arguments
    ///
    /// * `name` - Workflow name string.
    ///
    /// # Returns
    ///
    /// A new [`WorkflowRunner`] builder instance.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            log: None,
            cancel_token: None,
        }
    }

    /// Attaches a log writer to the runner.
    ///
    /// # Arguments
    ///
    /// * `log` - Log writer reference ([`LogWriter`]).
    ///
    /// # Returns
    ///
    /// Updated [`WorkflowRunner`] builder.
    pub fn with_log(mut self, log: &'a LogWriter) -> Self {
        self.log = Some(log);
        self
    }

    /// Attaches a cancellation token to the runner.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - Cancellation token reference ([`CancellationToken`]).
    ///
    /// # Returns
    ///
    /// Updated [`WorkflowRunner`] builder.
    pub fn with_cancel_token(mut self, cancel_token: &'a CancellationToken) -> Self {
        self.cancel_token = Some(cancel_token);
        self
    }

    /// Executes a multi-step workflow closure within a panic and cancellation boundary.
    ///
    /// # Arguments
    ///
    /// * `f` - Async closure taking a [`WorkflowContext`].
    ///
    /// # Returns
    ///
    /// A [`DockerResult`] containing the workflow output.
    pub async fn run<F, Fut, T>(self, f: F) -> DockerResult<T>
    where
        F: FnOnce(WorkflowContext<'a>) -> Fut,
        Fut: std::future::Future<Output = DockerResult<T>> + Send,
    {
        let journal = RollbackJournal::new();
        let context = WorkflowContext {
            name: self.name.clone(),
            journal: journal.clone(),
            log: self.log,
        };

        tracing::info!(workflow = %self.name, "starting workflow execution");

        let cancel_fut = async {
            if let Some(token) = self.cancel_token {
                token.cancelled().await;
            } else {
                futures_util::future::pending::<()>().await;
            }
        };

        let res = tokio::select! {
            res = AssertUnwindSafe(f(context)).catch_unwind() => res,
            _ = cancel_fut => {
                tracing::warn!(workflow = %self.name, "workflow cancelled by user; triggering rollback");
                journal.compensate(self.log).await;
                return Err(DockerError::BuildFailed("deployment was cancelled by user".to_string()));
            }
        };

        match res {
            Ok(Ok(val)) => {
                tracing::info!(workflow = %self.name, "workflow completed successfully");
                journal.clear();

                Ok(val)
            }
            Ok(Err(err)) => {
                tracing::warn!(workflow = %self.name, error = ?err, "workflow failed; triggering rollback");
                journal.compensate(self.log).await;

                Err(err)
            }
            Err(panic_payload) => {
                let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };

                tracing::error!(workflow = %self.name, panic = %panic_msg, "workflow panicked; triggering rollback");
                journal.compensate(self.log).await;

                Err(DockerError::Other(anyhow::anyhow!(
                    "Workflow panicked: {}",
                    panic_msg
                )))
            }
        }
    }
}
