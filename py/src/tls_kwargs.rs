//! Shared TLS kwarg parser used by RestClient and WebSocketClient
//! Python constructors.
//!
//! Presents a consistent 3-kwarg surface:
//! - `tls_ca_file: Optional[str | Path]` — ergonomic primary (matches
//!   requests/httpx `verify=<path>` convention)
//! - `tls_root_cert_pem: Optional[bytes]` — raw PEM bytes, power-user
//! - `tls_accept_invalid_certs: bool` — danger escape hatch
//!
//! `tls_ca_file` and `tls_root_cert_pem` are mutually exclusive.

use marketdata_core::TlsConfig;
use pyo3::prelude::*;

/// Parse the three TLS kwargs into a core `TlsConfig`. Emits a
/// `UserWarning` if `accept_invalid_certs` is true, so insecure usage is
/// flagged at construction time.
pub fn parse_tls_kwargs(
    py: Python<'_>,
    tls_ca_file: Option<String>,
    tls_root_cert_pem: Option<Vec<u8>>,
    tls_accept_invalid_certs: bool,
) -> PyResult<TlsConfig> {
    if tls_ca_file.is_some() && tls_root_cert_pem.is_some() {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "pass one of tls_ca_file or tls_root_cert_pem, not both",
        ));
    }

    let root_cert_pem = if let Some(path) = tls_ca_file {
        Some(std::fs::read(&path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!(
                "failed to read tls_ca_file {}: {}",
                path, e
            ))
        })?)
    } else {
        tls_root_cert_pem
    };

    if tls_accept_invalid_certs {
        let warnings = py.import("warnings")?;
        let warning_type = py.get_type::<pyo3::exceptions::PyUserWarning>();
        warnings.call_method1(
            "warn",
            (
                "INSECURE: TLS verification disabled — use tls_ca_file for production",
                warning_type,
            ),
        )?;
    }

    Ok(TlsConfig {
        root_cert_pem,
        accept_invalid_certs: tls_accept_invalid_certs,
    })
}
