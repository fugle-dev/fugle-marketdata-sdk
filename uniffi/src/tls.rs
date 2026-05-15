//! TLS configuration record for UniFFI bindings.
//!
//! Mirrors the shape of `marketdata_core::TlsConfig` in an FFI-friendly
//! form, following the same pattern as `ReconnectConfigRecord` and
//! `HealthCheckConfigRecord`.

/// Optional TLS customization exposed to foreign languages.
///
/// When all fields are default the SDK uses the OS trust store
/// (loaded by `rustls-native-certs`). Provide `root_cert_pem` to pin
/// an additional CA, or set `accept_invalid_certs` to disable all
/// verification (dev/testing only — exposes MITM risk).
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct TlsConfigRecord {
    /// PEM-encoded additional root CA bytes. Appended to the OS trust
    /// store; chains signed by either this CA or any OS-trusted root
    /// are accepted.
    pub root_cert_pem: Option<Vec<u8>>,

    /// Disable ALL TLS verification (chain + hostname + expiry).
    /// Equivalent to `curl -k` / `wscat --no-check`. Do not use in
    /// production.
    pub accept_invalid_certs: bool,
}

impl TlsConfigRecord {
    pub(crate) fn to_core(&self) -> marketdata_core::TlsConfig {
        marketdata_core::TlsConfig {
            root_cert_pem: self.root_cert_pem.clone(),
            accept_invalid_certs: self.accept_invalid_certs,
        }
    }
}
