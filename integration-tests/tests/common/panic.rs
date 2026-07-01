use near_workspaces::result::{ExecutionFailure, ExecutionResult, ExecutionSuccess};

/// Checks whether a transaction outcome panicked with a message containing `message`.
pub trait PanicFinder {
    fn has_panic(&self, message: &str) -> bool;
}

impl PanicFinder for Result<ExecutionSuccess, ExecutionFailure> {
    fn has_panic(&self, message: &str) -> bool {
        match self {
            Ok(ok) => ok.has_panic(message),
            Err(err) => err.has_panic(message),
        }
    }
}

impl<T> PanicFinder for ExecutionResult<T> {
    fn has_panic(&self, message: &str) -> bool {
        self.outcomes().into_iter().any(|outcome| {
            outcome
                .clone()
                .into_result()
                .err()
                .is_some_and(|err| format!("{err:?}").contains(message))
        })
    }
}
