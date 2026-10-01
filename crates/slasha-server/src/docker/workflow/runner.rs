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
    /// The undo is registered before the action runs, so a rollback also cleans up
    /// after a step that failed or was cancelled part-way, having created some of
    /// what it was meant to. Every undo must therefore be idempotent, tolerate the
    /// action having done nothing, and touch only what the action would create.
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

        self.journal.push(step_name.to_string(), undo);

        action.await
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

        // The workflow future is dropped when select! returns, which kills any
        // child process it spawned with kill_on_drop, so a cancelled step cannot
        // keep creating resources after its undo has run.
        let res = tokio::select! {
            res = AssertUnwindSafe(f(context)).catch_unwind() => Some(res),
            _ = cancel_fut => None,
        };

        let Some(res) = res else {
            tracing::warn!(workflow = %self.name, "workflow cancelled by user; triggering rollback");
            journal.compensate(self.log).await;
            return Err(DockerError::BuildFailed(
                "deployment was cancelled by user".to_string(),
            ));
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use tokio_util::sync::CancellationToken;

    use super::WorkflowRunner;
    use crate::docker::DockerError;

    fn counter() -> (
        Arc<AtomicUsize>,
        impl Fn() -> futures_util::future::BoxFuture<'static, ()> + Clone,
    ) {
        let count = Arc::new(AtomicUsize::new(0));
        let undo = {
            let count = count.clone();
            move || {
                let count = count.clone();
                Box::pin(async move {
                    count.fetch_add(1, Ordering::SeqCst);
                }) as futures_util::future::BoxFuture<'static, ()>
            }
        };
        (count, undo)
    }

    #[tokio::test]
    async fn a_step_that_fails_part_way_is_undone() {
        let (undone, undo) = counter();

        let result = WorkflowRunner::new("test")
            .run(|wf| async move {
                wf.step("first", async { Ok::<_, DockerError>(()) }, undo())
                    .await?;
                wf.step(
                    "second",
                    async { Err::<(), _>(DockerError::Other(anyhow::anyhow!("half done"))) },
                    undo(),
                )
                .await
            })
            .await;

        assert!(result.is_err());
        assert_eq!(undone.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_successful_workflow_undoes_nothing() {
        let (undone, undo) = counter();

        WorkflowRunner::new("test")
            .run(|wf| async move {
                wf.step("only", async { Ok::<_, DockerError>(()) }, undo())
                    .await
            })
            .await
            .unwrap();

        assert_eq!(undone.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_step_cancelled_mid_action_is_undone() {
        let (undone, undo) = counter();
        let token = CancellationToken::new();

        let run = WorkflowRunner::new("test")
            .with_cancel_token(&token)
            .run(|wf| async move {
                wf.step(
                    "slow",
                    async {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        Ok::<_, DockerError>(())
                    },
                    undo(),
                )
                .await
            });

        let (result, ()) = tokio::join!(run, async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            token.cancel();
        });

        assert!(result.is_err());
        assert_eq!(undone.load(Ordering::SeqCst), 1);
    }
}
