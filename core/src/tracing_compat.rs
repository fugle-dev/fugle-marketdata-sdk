//! Tracing compatibility layer.
//!
//! Re-exports `tracing`'s `debug!`, `info!`, `warn!`, `error!` macros when
//! the `tracing` feature is enabled and provides no-op stand-ins when it
//! is not. Callers `use crate::tracing_compat::{debug, info, warn, error}`
//! and write feature-agnostic code; the no-op variants compile to nothing.
//!
//! The `#[tracing::instrument(...)]` attribute is gated separately at the
//! call site via `#[cfg_attr(feature = "tracing", tracing::instrument(...))]`
//! because attributes cannot be re-exported.

#[cfg(feature = "tracing")]
pub(crate) use tracing::{debug, error, info, warn};

#[cfg(not(feature = "tracing"))]
mod noop {
    #[macro_export]
    #[doc(hidden)]
    macro_rules! __tracing_noop {
        ($($_:tt)*) => {};
    }

    pub(crate) use crate::__tracing_noop as debug;
    pub(crate) use crate::__tracing_noop as error;
    pub(crate) use crate::__tracing_noop as info;
    pub(crate) use crate::__tracing_noop as warn;
}

#[cfg(not(feature = "tracing"))]
pub(crate) use noop::{debug, error, info, warn};
