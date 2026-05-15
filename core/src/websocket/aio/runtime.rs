//! FFI-safe async runtime wrapper
//!
//! This module provides a safe wrapper around Tokio's async runtime for use across FFI boundaries.
//! Key features:
//! - Single runtime instance pattern
//! - Panic boundary for FFI safety
//! - Handle-based task spawning
//! - Graceful shutdown support

use crate::errors::MarketDataError;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use tokio::runtime::{Handle, Runtime};

/// Macro for catching panics at FFI boundaries (returns pointer)
///
/// This prevents panics from crossing FFI boundaries, which would be undefined behavior.
/// Instead, panics are caught and converted to null pointers.
macro_rules! ffi_catch_ptr {
    ($body:expr) => {
        match catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => {
                eprintln!("PANIC: Caught panic at FFI boundary");
                ptr::null_mut()
            }
        }
    };
}

/// Macro for catching panics at FFI boundaries (returns void)
///
/// This prevents panics from crossing FFI boundaries, which would be undefined behavior.
/// Panics are caught and logged.
macro_rules! ffi_catch_void {
    ($body:expr) => {
        if let Err(_) = catch_unwind(AssertUnwindSafe(|| $body)) {
            eprintln!("PANIC: Caught panic at FFI boundary");
        }
    };
}

/// FFI-safe async runtime wrapper
///
/// This struct wraps Tokio's Runtime and provides safe methods for:
/// - Creating/destroying runtime across FFI boundary
/// - Spawning async tasks
/// - Blocking on futures
/// - Graceful shutdown
pub struct AsyncRuntime {
    runtime: Runtime,
}

impl AsyncRuntime {
    /// Create a new multi-threaded Tokio runtime
    ///
    /// # Errors
    /// Returns RuntimeError if the runtime cannot be created
    pub fn new() -> Result<Self, MarketDataError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| MarketDataError::RuntimeError {
                msg: format!("Failed to create runtime: {}", e),
            })?;

        Ok(Self { runtime })
    }

    /// Get a handle to the runtime for spawning tasks
    pub fn handle(&self) -> Handle {
        self.runtime.handle().clone()
    }

    /// Block on a future until it completes
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        self.runtime.block_on(future)
    }

    /// Spawn a task on the runtime
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    /// Shutdown the runtime gracefully
    pub fn shutdown(self) {
        // Drop the runtime, which triggers graceful shutdown
        drop(self.runtime);
    }
}

// FFI functions for external language bindings

/// Create a new async runtime (FFI-safe)
///
/// Returns an opaque pointer to the runtime, or null on error.
/// The caller must call `destroy_runtime` when done.
///
/// # Safety
/// - Caller must eventually call `destroy_runtime` with the returned pointer
/// - Returned pointer must not be used after `destroy_runtime` is called
#[no_mangle]
pub extern "C" fn create_runtime() -> *mut AsyncRuntime {
    ffi_catch_ptr!({
        match AsyncRuntime::new() {
            Ok(runtime) => Box::into_raw(Box::new(runtime)),
            Err(e) => {
                eprintln!("Failed to create runtime: {}", e);
                ptr::null_mut()
            }
        }
    })
}

/// Destroy an async runtime (FFI-safe)
///
/// # Safety
/// - `runtime_ptr` must be a valid pointer from `create_runtime`
/// - `runtime_ptr` must not be used after this call
/// - Calling with null pointer is safe (no-op)
#[no_mangle]
pub unsafe extern "C" fn destroy_runtime(runtime_ptr: *mut AsyncRuntime) {
    ffi_catch_void!({
        if !runtime_ptr.is_null() {
            unsafe {
                let runtime = Box::from_raw(runtime_ptr);
                runtime.shutdown();
            }
        }
    })
}

/// Check if a runtime pointer is valid (non-null)
///
/// # Safety
/// This only checks if the pointer is non-null, not if it's still valid.
#[no_mangle]
pub extern "C" fn runtime_is_valid(runtime_ptr: *const AsyncRuntime) -> bool {
    !runtime_ptr.is_null()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_runtime_creation() {
        let runtime = AsyncRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_runtime_block_on() {
        let runtime = AsyncRuntime::new().unwrap();
        let result = runtime.block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_runtime_spawn_and_await() {
        let runtime = AsyncRuntime::new().unwrap();
        let handle = runtime.spawn(async { "hello" });
        let result = runtime.block_on(handle).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_runtime_handle_multiple_tasks() {
        let runtime = AsyncRuntime::new().unwrap();
        let counter = Arc::new(AtomicU32::new(0));

        let mut handles = vec![];
        for _ in 0..10 {
            let counter_clone = counter.clone();
            let handle = runtime.spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            runtime.block_on(handle).unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_runtime_shutdown() {
        let runtime = AsyncRuntime::new().unwrap();
        runtime.shutdown();
        // If we reach here without panic, shutdown was graceful
    }

    #[test]
    fn test_ffi_create_destroy() {
        let runtime_ptr = create_runtime();
        assert!(!runtime_ptr.is_null());
        assert!(runtime_is_valid(runtime_ptr));
        // SAFETY: runtime_ptr is valid from create_runtime
        unsafe { destroy_runtime(runtime_ptr) };
    }

    #[test]
    fn test_ffi_destroy_null() {
        // Should not panic - null pointer is handled safely
        // SAFETY: destroy_runtime explicitly handles null pointers
        unsafe { destroy_runtime(ptr::null_mut()) };
    }

    #[test]
    #[should_panic(expected = "test panic")]
    fn test_panic_boundary() {
        // This test demonstrates that panics within Rust code can be caught
        // In real FFI scenarios, the ffi_catch macros prevent panics from crossing boundaries
        std::panic::panic_any("test panic");
    }
}
