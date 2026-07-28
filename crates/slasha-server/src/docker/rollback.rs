use futures_util::future::BoxFuture;

type CompensationFn = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;

/// Legacy synchronous rollback journal for tracking compensation actions.
pub struct Rollback {
    steps: Vec<CompensationFn>,
}

impl Default for Rollback {
    fn default() -> Self {
        Self::new()
    }
}

impl Rollback {
    /// Creates a new empty [`Rollback`] journal.
    ///
    /// # Returns
    ///
    /// A new [`Rollback`] instance.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Registers a compensation closure step in the rollback stack.
    ///
    /// # Arguments
    ///
    /// * `f` - Compensation closure returning a boxed future.
    pub fn register(&mut self, f: impl FnOnce() -> BoxFuture<'static, ()> + Send + 'static) {
        self.steps.push(Box::new(f));
    }

    /// Executes all registered rollback steps in reverse order.
    pub async fn execute(self) {
        for step in self.steps.into_iter().rev() {
            step().await;
        }
    }

    /// Disarms and drops the rollback journal without executing compensation steps.
    pub fn disarm(self) {
        drop(self);
    }
}
