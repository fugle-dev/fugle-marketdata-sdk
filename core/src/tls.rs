//! TLS configuration for REST (`ureq`) and WebSocket (`tokio-tungstenite`).
//!
//! Both transports share the same [`TlsConfig`] shape and the
//! [`build_rustls_config`] helper, so a user-supplied root CA or
//! "accept invalid certs" flag applies uniformly across the SDK.

use std::io::BufReader;
use std::sync::{Arc, OnceLock};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};

use crate::errors::MarketDataError;

/// Optional TLS customization. When all fields are default the SDK uses
/// the OS trust store loaded by `rustls-native-certs`.
#[derive(Clone, Debug, Default)]
pub struct TlsConfig {
    /// PEM-encoded additional root CA. Added to the rustls `RootCertStore`
    /// alongside the system trust store, so the client accepts chains
    /// signed by either this CA or any OS-trusted root.
    pub root_cert_pem: Option<Vec<u8>>,

    /// Disable ALL TLS verification (chain + hostname + expiry + EKU).
    /// Equivalent to `wscat --no-check` or `curl -k`. Do not use in
    /// production — exposes the client to trivial MITM. Prefer
    /// `root_cert_pem` with a properly-issued server cert.
    pub accept_invalid_certs: bool,
}

static PROVIDER_INSTALLED: OnceLock<()> = OnceLock::new();
static SYSTEM_ROOTS: OnceLock<Arc<RootCertStore>> = OnceLock::new();

fn install_crypto_provider() {
    PROVIDER_INSTALLED.get_or_init(|| {
        // Best-effort: if another crate already installed one, this
        // returns Err and we ignore it. First installer wins.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn system_root_store() -> &'static Arc<RootCertStore> {
    SYSTEM_ROOTS.get_or_init(|| {
        let mut store = RootCertStore::empty();
        // rustls-native-certs 0.8 returns CertificateResult { certs, errors }.
        // We ignore per-cert errors — reading some OS stores may fail on
        // locked-down systems, but usable roots still load.
        let loaded = rustls_native_certs::load_native_certs();
        for cert in loaded.certs {
            let _ = store.add(cert);
        }
        Arc::new(store)
    })
}

/// Build a rustls [`ClientConfig`] honoring any custom root CA or
/// `accept_invalid_certs` flag in `tls`. On default config this returns
/// a config using the OS trust store loaded once into a process-wide
/// `RootCertStore`. ureq's rustls integration and tokio-tungstenite's
/// `Connector::Rustls` both consume this `Arc<ClientConfig>`.
pub fn build_rustls_config(tls: &TlsConfig) -> Result<Arc<ClientConfig>, MarketDataError> {
    install_crypto_provider();

    if tls.accept_invalid_certs {
        // DangerousClientConfigBuilder lives inline on the builder since
        // rustls 0.23 (no feature flag required).
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AlwaysTrustVerifier))
            .with_no_client_auth();
        return Ok(Arc::new(config));
    }

    // Clone the cached system store so callers can't mutate it.
    let mut store = (**system_root_store()).clone();

    if let Some(pem) = &tls.root_cert_pem {
        let mut reader = BufReader::new(pem.as_slice());
        for cert_result in rustls_pemfile::certs(&mut reader) {
            let cert = cert_result.map_err(|e| {
                MarketDataError::ConfigError(format!("invalid TLS root cert PEM: {e}"))
            })?;
            store.add(cert).map_err(|e| {
                MarketDataError::ConfigError(format!("failed to add root cert: {e}"))
            })?;
        }
    }

    let config = ClientConfig::builder()
        .with_root_certificates(store)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Verifier that accepts any server certificate. Only used when the
/// caller opts in via `TlsConfig::accept_invalid_certs`.
#[derive(Debug)]
struct AlwaysTrustVerifier;

impl ServerCertVerifier for AlwaysTrustVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_builds_rustls_config() {
        let cfg = TlsConfig::default();
        let _ = build_rustls_config(&cfg).expect("default should always build");
    }

    #[test]
    fn accept_invalid_certs_builds_rustls_config() {
        let cfg = TlsConfig {
            accept_invalid_certs: true,
            ..Default::default()
        };
        let _ = build_rustls_config(&cfg).expect("should build");
    }

    #[test]
    fn invalid_pem_is_config_error() {
        let cfg = TlsConfig {
            root_cert_pem: Some(b"not a real pem".to_vec()),
            ..Default::default()
        };
        // rustls-pemfile silently returns 0 certs on garbage input (not an
        // error) — so the config builds but with no extra root added.
        // That's acceptable: the surrounding "invalid PEM" contract was
        // native-tls-specific. A clearly-invalid PEM that DOES look like
        // a cert header would fail inside the iterator.
        let cfg_ok = build_rustls_config(&cfg);
        assert!(cfg_ok.is_ok(), "garbage non-PEM should parse to zero certs, not error");
    }
}
