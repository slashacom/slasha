use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::future::BoxFuture;
use tokio::time::timeout;

use crate::logs::LogHandle;

pub struct Compensation {
    pub name: String,
    pub step: BoxFuture<'static, ()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalState {
    Active,
    Committed,
    RolledBack,
}

struct Inner {
    state: JournalState,
    compensations: Vec<Compensation>,
}

/// Thread-safe journal tracking compensation steps registered during workflow execution.
#[derive(Clone)]
pub struct RollbackJournal {
    inner: Arc<Mutex<Inner>>,
}

impl Default for RollbackJournal {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackJournal {
    /// Creates a new empty [`RollbackJournal`].
    ///
    /// # Returns
    ///
    /// A new [`RollbackJournal`] instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                state: JournalState::Active,
                compensations: Vec::new(),
            })),
        }
    }

    /// Registers an async compensation undo action in the journal.
    ///
    /// # Arguments
    ///
    /// * `name` - Descriptive step name string.
    /// * `undo` - Async closure or future representing the undo operation.
    pub fn push<Fut>(&self, name: impl Into<String>, undo: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        if guard.state == JournalState::Active {
            guard.compensations.push(Compensation {
                name: name.into(),
                step: Box::pin(undo),
            });
        }
    }

    /// Executes all registered compensation steps in reverse order (LIFO).
    ///
    /// # Arguments
    ///
    /// * `log` - Optional log handle reference ([`LogHandle`]).
    pub async fn compensate(&self, log: Option<&LogHandle>) {
        let compensations = {
            let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());

            if guard.state != JournalState::Active {
                return;
            }

            guard.state = JournalState::RolledBack;
            std::mem::take(&mut guard.compensations)
        };

        execute_compensations(compensations, log).await;
    }

    /// Clears all registered compensation steps without executing them.
    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.state = JournalState::Committed;
        guard.compensations.clear();
    }
}

/// Executes a vector of compensation steps sequentially in reverse order.
///
/// # Arguments
///
/// * `compensations` - Vector of compensation steps ([`Compensation`]).
/// * `log` - Optional log handle reference ([`LogHandle`]).
async fn execute_compensations(compensations: Vec<Compensation>, log: Option<&LogHandle>) {
    if compensations.is_empty() {
        return;
    }

    if let Some(log) = log {
        let _ = log.send("Rolling back changes...".to_string()).await;
    }

    for comp in compensations.into_iter().rev() {
        tracing::info!(step = %comp.name, "executing rollback step");
        if let Some(log) = log {
            let _ = log.send(format!("Rolling back step: {}", comp.name)).await;
        }

        let timeout_res = timeout(Duration::from_secs(30), comp.step).await;

        if timeout_res.is_err() {
            tracing::warn!(step = %comp.name, "rollback step timed out after 30s");
            if let Some(log) = log {
                let _ = log
                    .send(format!(
                        "Warning: rollback step '{}' timed out after 30s",
                        comp.name
                    ))
                    .await;
            }
        }
    }
}
