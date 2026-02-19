use std::panic::{self, AssertUnwindSafe};

/// Catch panics and return a default value on panic.
pub fn catch_panic_or_default<F, T>(f: F, context: &str, default: T) -> T
where
    F: FnOnce() -> T,
{
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => {
            log::error!("Panic caught in {context}, returning default");
            default
        }
    }
}

/// Catch panics and return Option<T>.
pub fn catch_panic<F, T>(f: F, context: &str) -> Option<T>
where
    F: FnOnce() -> T,
{
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => Some(result),
        Err(_) => {
            log::error!("Panic caught in {context}");
            None
        }
    }
}
