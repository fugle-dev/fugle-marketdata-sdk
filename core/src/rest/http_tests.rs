//! End-to-end behaviour of the REST transport against a local HTTP server.
//!
//! These pin down what callers observe for each kind of HTTP outcome, so the
//! underlying HTTP client can change without silently changing error variants,
//! retry behaviour, headers or proxy handling.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::errors::MarketDataError;
use crate::rest::{Auth, RestClient, RetryPolicy};
use crate::tls::TlsConfig;

const QUOTE: &str = r#"{"date":"2026-01-30","type":"EQUITY","exchange":"TWSE","market":"TSE","symbol":"2330","name":"台積電"}"#;

fn raw(status: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn gzip_raw(body: &str) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(body.as_bytes()).unwrap();
    let payload = enc.finish().unwrap();
    let mut out = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n",
        payload.len()
    )
    .into_bytes();
    out.extend_from_slice(&payload);
    out
}

/// A scripted server: the n-th request gets `responses[n]` (the last one
/// repeats). `None` means accept the request and never answer.
struct Server {
    base: String,
    heads: Arc<Mutex<Vec<String>>>,
}

fn server(responses: Vec<Option<Vec<u8>>>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(responses);
    let seen = heads.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (responses, seen) = (responses.clone(), seen.clone());
            thread::spawn(move || handle(stream, &responses, &seen));
        }
    });
    Server { base: format!("http://127.0.0.1:{port}/marketdata"), heads }
}

fn handle(mut stream: TcpStream, responses: &[Option<Vec<u8>>], seen: &Mutex<Vec<String>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..end]).into_owned();
            buf.drain(..end + 4);
            let index = {
                let mut seen = seen.lock().unwrap();
                seen.push(head);
                seen.len() - 1
            };
            match &responses[index.min(responses.len() - 1)] {
                Some(bytes) => {
                    if stream.write_all(bytes).is_err() {
                        return;
                    }
                }
                None => {
                    thread::sleep(Duration::from_secs(5));
                    return;
                }
            }
        }
    }
}

fn client(base: &str) -> RestClient {
    RestClient::new(Auth::ApiKey("test-key".into())).base_url(base)
}

fn quote(client: &RestClient) -> Result<serde_json::Value, MarketDataError> {
    client.stock().intraday().quote().symbol("2330").send()
}

#[test]
fn success_decodes_json() {
    let srv = server(vec![Some(raw("200 OK", QUOTE))]);
    let q = quote(&client(&srv.base)).expect("200 should decode");
    assert_eq!(q["symbol"], "2330");
}

#[test]
fn gzip_success_decodes_json() {
    let srv = server(vec![Some(gzip_raw(QUOTE))]);
    let q = quote(&client(&srv.base)).expect("gzip 200 should decode");
    assert_eq!(q["symbol"], "2330");
}

#[test]
fn sends_auth_header_and_accepts_gzip() {
    for (auth, header) in [
        (Auth::ApiKey("k-123".into()), "x-api-key: k-123"),
        (Auth::BearerToken("t-456".into()), "authorization: bearer t-456"),
        (Auth::SdkToken("s-789".into()), "x-sdk-token: s-789"),
    ] {
        let srv = server(vec![Some(raw("200 OK", QUOTE))]);
        let c = RestClient::new(auth).base_url(&srv.base);
        quote(&c).unwrap();
        let head = srv.heads.lock().unwrap()[0].to_ascii_lowercase();
        assert!(head.contains(header), "missing `{header}` in:\n{head}");
        assert!(head.contains("accept-encoding:") && head.contains("gzip"), "missing gzip accept-encoding in:\n{head}");
        assert!(head.starts_with("get /marketdata/v1.0/stock/intraday/quote/2330 "), "unexpected request line:\n{head}");
    }
}

#[test]
fn credential_with_invalid_header_characters_is_config_error() {
    let srv = server(vec![Some(raw("200 OK", QUOTE))]);
    let c = RestClient::new(Auth::ApiKey("bad\nkey".into())).base_url(&srv.base);
    assert!(matches!(quote(&c), Err(MarketDataError::ConfigError(_))));
    assert!(srv.heads.lock().unwrap().is_empty(), "must not reach the network");
}

#[test]
fn status_401_and_403_are_auth_errors_with_body() {
    for status in ["401 Unauthorized", "403 Forbidden"] {
        let srv = server(vec![Some(raw(status, r#"{"message":"Unauthorized"}"#))]);
        match quote(&client(&srv.base)) {
            Err(MarketDataError::AuthError { msg, .. }) => assert!(msg.contains("Unauthorized"), "{msg}"),
            other => panic!("{status}: expected AuthError, got {other:?}"),
        }
    }
}

#[test]
fn error_info_carries_status_body_and_headers() {
    let body = r#"{"message":"Unauthorized","statusCode":401}"#;
    for status in ["401 Unauthorized", "404 Not Found"] {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nX-Request-Id: req-1\r\nX-RateLimit-Remaining: 0\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let srv = server(vec![Some(response.into_bytes())]);
        let info = quote(&client(&srv.base)).unwrap_err().info();
        assert_eq!(info.status, Some(status[..3].parse().unwrap()), "{status}");
        assert_eq!(info.body.as_deref(), Some(body), "{status}");
        assert_eq!(info.request_id.as_deref(), Some("req-1"), "{status}");
        assert_eq!(info.headers.get("x-ratelimit-remaining").map(String::as_str), Some("0"), "{status}");
        assert_eq!(info.headers.get("content-type").map(String::as_str), Some("application/json"), "{status}");
    }
}

#[test]
fn other_statuses_are_api_errors_with_body() {
    for (status, code) in [("404 Not Found", 404), ("429 Too Many Requests", 429), ("500 Internal Server Error", 500)] {
        let srv = server(vec![Some(raw(status, r#"{"message":"nope"}"#))]);
        match quote(&client(&srv.base)) {
            Err(MarketDataError::ApiError { status, message, .. }) => {
                assert_eq!(status, code);
                assert_eq!(message, r#"{"message":"nope"}"#);
            }
            other => panic!("{status}: expected ApiError, got {other:?}"),
        }
    }
}

#[test]
fn invalid_json_is_other_error() {
    let srv = server(vec![Some(raw("200 OK", "not json"))]);
    assert!(matches!(quote(&client(&srv.base)), Err(MarketDataError::Other(_))));
}

#[test]
fn connection_refused_is_connection_error() {
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let c = client(&format!("http://127.0.0.1:{port}/marketdata"));
    match quote(&c) {
        Err(e @ MarketDataError::ConnectionError { .. }) => assert!(e.is_retryable()),
        other => panic!("expected ConnectionError, got {other:?}"),
    }
}

#[test]
fn unanswered_request_times_out() {
    let srv = server(vec![None]);
    let c = RestClient::with_tls_and_timeout(Auth::ApiKey("k".into()), TlsConfig::default(), Duration::from_millis(300))
        .unwrap()
        .base_url(&srv.base);
    let started = Instant::now();
    match quote(&c) {
        Err(e @ MarketDataError::TimeoutError { .. }) => assert!(e.is_retryable()),
        other => panic!("expected TimeoutError, got {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(4), "timeout was not applied");
}

#[test]
fn retry_policy_resends_after_server_error() {
    let srv = server(vec![Some(raw("503 Service Unavailable", "busy")), Some(raw("200 OK", QUOTE))]);
    let policy = RetryPolicy::new(3, Duration::from_millis(1), Duration::from_millis(5));
    let c = client(&srv.base).with_retry(policy);
    let q = quote(&c).expect("second attempt should succeed");
    assert_eq!(q["symbol"], "2330");
    assert_eq!(srv.heads.lock().unwrap().len(), 2);
}

#[test]
fn proxy_environment_variables_are_ignored() {
    // Serialise with any other test that touches proxy variables.
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let srv = server(vec![Some(raw("200 OK", QUOTE))]);
    let vars = ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"];
    let saved: Vec<_> = vars.iter().map(|v| (v, std::env::var_os(v))).collect();
    for v in vars {
        std::env::set_var(v, "http://127.0.0.1:1");
    }
    let result = quote(&client(&srv.base));
    for (v, old) in saved {
        match old {
            Some(val) => std::env::set_var(v, val),
            None => std::env::remove_var(v),
        }
    }
    result.expect("requests must not be routed through proxy env vars");
}

// ---------------------------------------------------------------------------
// HTTPS: trust store, extra root CA, and disabled verification.

struct Pki {
    ca_pem: String,
    server_config: Arc<rustls::ServerConfig>,
}

/// A throwaway CA plus a `127.0.0.1` / `localhost` server certificate it signed.
fn pki() -> Pki {
    use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    let ca_issuer = rcgen::Issuer::new(ca_params, ca_key);

    let leaf_key = KeyPair::generate().unwrap();
    let leaf_params = CertificateParams::new(vec!["127.0.0.1".into(), "localhost".into()]).unwrap();
    let leaf_cert = leaf_params.signed_by(&leaf_key, &ca_issuer).unwrap();

    let key_der = rustls::pki_types::PrivateKeyDer::try_from(leaf_key.serialize_der()).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server_config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![leaf_cert.der().clone()], key_der)
        .unwrap();
    Pki { ca_pem: ca_cert.pem(), server_config: Arc::new(server_config) }
}

/// HTTPS server answering every request with `QUOTE`.
fn https_server(config: Arc<rustls::ServerConfig>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for tcp in listener.incoming().flatten() {
            let config = config.clone();
            thread::spawn(move || {
                let Ok(conn) = rustls::ServerConnection::new(config) else { return };
                let mut tls = rustls::StreamOwned::new(conn, tcp);
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = match tls.read(&mut chunk) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        buf.drain(..end + 4);
                        if tls.write_all(&raw("200 OK", QUOTE)).is_err() || tls.flush().is_err() {
                            return;
                        }
                    }
                }
            });
        }
    });
    format!("https://127.0.0.1:{port}/marketdata")
}

fn https_client(tls: TlsConfig, base: &str) -> RestClient {
    RestClient::with_tls(Auth::ApiKey("k".into()), tls).unwrap().base_url(base)
}

#[test]
fn https_rejects_certificate_from_unknown_ca() {
    let pki = pki();
    let base = https_server(pki.server_config);
    match quote(&https_client(TlsConfig::default(), &base)) {
        Err(MarketDataError::ConnectionError { .. }) => {}
        other => panic!("expected a TLS verification failure, got {other:?}"),
    }
}

#[test]
fn https_accepts_certificate_from_extra_root_ca() {
    let pki = pki();
    let base = https_server(pki.server_config);
    let tls = TlsConfig { root_cert_pem: Some(pki.ca_pem.into_bytes()), ..Default::default() };
    let q = quote(&https_client(tls, &base)).expect("extra root CA should be trusted");
    assert_eq!(q["symbol"], "2330");
}

#[test]
fn https_accept_invalid_certs_skips_verification() {
    let pki = pki();
    let base = https_server(pki.server_config);
    let tls = TlsConfig { accept_invalid_certs: true, ..Default::default() };
    let q = quote(&https_client(tls, &base)).expect("verification disabled");
    assert_eq!(q["symbol"], "2330");
}

/// Talks to the real API, so it is ignored by default:
/// `cargo test -p fugle-marketdata-core --lib live_https -- --ignored`.
/// Without credentials the API answers 401, which proves the TLS handshake
/// succeeded against the OS trust store.
#[test]
#[ignore = "requires network access to api.fugle.tw"]
fn live_https_uses_os_trust_store() {
    let c = RestClient::new(Auth::ApiKey("invalid".into()));
    match quote(&c) {
        Err(MarketDataError::AuthError { .. }) => {}
        other => panic!("expected 401 AuthError from the live API, got {other:?}"),
    }
}

