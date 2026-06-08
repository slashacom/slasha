use futures_util::future::BoxFuture;

type CompensationFn = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;

pub struct Rollback {
    steps: Vec<CompensationFn>,
}

impl Default for Rollback {
    fn default() -> Self {
        Self::new()
    }
}

impl Rollback {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn register(&mut self, f: impl FnOnce() -> BoxFuture<'static, ()> + Send + 'static) {
        self.steps.push(Box::new(f));
    }

    pub async fn execute(self) {
        for step in self.steps.into_iter().rev() {
            step().await;
        }
    }

    pub fn disarm(self) {
        drop(self);
    }
}
