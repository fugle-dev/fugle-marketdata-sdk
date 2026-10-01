//! The credential a WebSocket client authenticates with, replaceable while
//! the client lives (#322).

use crate::models::AuthRequest;
use crate::{Auth, MarketDataError};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

/// The credential sent in the auth frame of every connection attempt.
///
/// Clones share one credential: a value set through any of them is the one
/// the next attempt sends. A client starts with its own handle, built from
/// [`ConnectionConfig::auth`](crate::websocket::ConnectionConfig::auth).
/// Code that builds a new client for each connection keeps one handle and
/// hands it to each client with `use_credentials_handle()`, so a credential
/// set between connections is not lost.
///
/// Setting a credential changes only later connection attempts — a
/// `connect()`, an automatic reconnect, a `reconnect()`. A connection that
/// is already authenticated is not authenticated again.
#[derive(Clone)]
pub struct CredentialsHandle {
    inner: Arc<RwLock<AuthRequest>>,
}

impl CredentialsHandle {
    /// A handle holding `auth`, which takes the type of
    /// `ConnectionConfig::auth` so a client-requested heartbeat interval can
    /// come along; [`set`](Self::set) takes only the credential.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] unless exactly one non-blank
    /// credential is set, so no handle holds one a connection would send
    /// unchecked.
    pub fn new(auth: AuthRequest) -> Result<Self, MarketDataError> {
        auth.validate()?;
        Ok(Self::unchecked(auth))
    }

    /// A client's own handle, built from `ConnectionConfig::auth`, which its
    /// `connect()` checks before the first attempt.
    fn unchecked(auth: AuthRequest) -> Self {
        Self { inner: Arc::new(RwLock::new(auth)) }
    }

    /// Replace the credential, which may be of another kind than the one it
    /// replaces. The client-requested heartbeat interval is kept.
    ///
    /// # Errors
    ///
    /// Returns [`MarketDataError::ConfigError`] for a blank credential; the
    /// held one is then left as it was.
    pub fn set(&self, auth: Auth) -> Result<(), MarketDataError> {
        auth.validate()?;
        let mut next = AuthRequest::from(auth);
        let mut current = self.inner.write().unwrap_or_else(PoisonError::into_inner);
        next.heartbeat_interval_ms = current.heartbeat_interval_ms;
        *current = next;
        Ok(())
    }

    /// The credential the next connection attempt sends.
    pub(crate) fn current(&self) -> AuthRequest {
        self.inner.read().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl std::fmt::Debug for CredentialsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `AuthRequest`'s `Debug` redacts the secret.
        f.debug_tuple("CredentialsHandle").field(&self.current()).finish()
    }
}

/// A client's handle, which `use_credentials_handle()` may swap for another.
pub(crate) struct CredentialsSlot(Mutex<CredentialsHandle>);

impl CredentialsSlot {
    pub(crate) fn new(auth: AuthRequest) -> Self {
        Self(Mutex::new(CredentialsHandle::unchecked(auth)))
    }

    pub(crate) fn handle(&self) -> CredentialsHandle {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub(crate) fn use_handle(&self, handle: &CredentialsHandle) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = handle.clone();
    }

    /// The credential the next connection attempt sends.
    pub(crate) fn current(&self) -> AuthRequest {
        self.handle().current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_replaces_the_kind_and_keeps_the_heartbeat_interval() {
        let mut auth = AuthRequest::with_api_key("a");
        auth.heartbeat_interval_ms = Some(5_000);
        let handle = CredentialsHandle::new(auth).unwrap();
        let shared = handle.clone();

        shared.set(Auth::SdkToken("b".into())).unwrap();

        let current = handle.current();
        assert_eq!(current.apikey, None);
        assert_eq!(current.sdk_token.as_deref(), Some("b"));
        assert_eq!(current.heartbeat_interval_ms, Some(5_000));
    }

    #[test]
    fn a_blank_credential_is_refused_and_the_held_one_kept() {
        let handle = CredentialsHandle::new(AuthRequest::with_token("a")).unwrap();

        let err = handle.set(Auth::ApiKey("  ".into())).unwrap_err();

        assert_eq!(err.to_error_code(), crate::error_code::CONFIG);
        assert_eq!(handle.current().token.as_deref(), Some("a"));
    }

    #[test]
    fn new_refuses_a_blank_or_ambiguous_credential() {
        let mut both = AuthRequest::with_api_key("a");
        both.token = Some("b".into());
        for auth in [AuthRequest::with_sdk_token(" "), both] {
            let err = CredentialsHandle::new(auth).unwrap_err();
            assert_eq!(err.to_error_code(), crate::error_code::CONFIG);
        }
    }

    #[test]
    fn a_slot_follows_the_handle_it_was_given() {
        let slot = CredentialsSlot::new(AuthRequest::with_api_key("a"));
        let handle = CredentialsHandle::new(AuthRequest::with_api_key("b")).unwrap();

        slot.use_handle(&handle);
        handle.set(Auth::BearerToken("c".into())).unwrap();

        assert_eq!(slot.current().token.as_deref(), Some("c"));
    }

    #[test]
    fn debug_redacts_the_secret() {
        let handle = CredentialsHandle::new(AuthRequest::with_sdk_token("secret")).unwrap();
        assert!(!format!("{handle:?}").contains("secret"));
    }
}
