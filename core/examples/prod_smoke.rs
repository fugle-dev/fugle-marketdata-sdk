//! Prod-environment smoke sweep.
//!
//! Hits every REST endpoint + every WebSocket channel against **production**
//! with `FUGLE_API_KEY`, fully deserialising each response, and emits one
//! JSON record per probe to stdout. Surfaces decode landmines (server sends a
//! field in a shape the Rust type can't accept — e.g. `endSession: "1"` vs
//! `Option<i32>`) so they can be fixed in a follow-up patch.
//!
//! # Run
//! ```bash
//! export FUGLE_API_KEY="your-prod-key"
//! cargo run -p fugle-marketdata-core --example prod_smoke --features tokio-comp \
//!   | tee report.jsonl
//! # triage failures:
//! jq -c 'select(.outcome != "Pass" and .outcome != "NoData")' report.jsonl
//! ```
//!
//! Exit code is non-zero if any probe is not `Pass`/`NoData`.

use marketdata_core::{
    aio::WebSocketClient as AsyncWs, parse_channel_data, AuthRequest, Auth, Channel,
    ConnectionConfig, FutOptChannel, FutOptType, MarketDataError, RestClient,
    StockSubscription,
};
use marketdata_core::websocket::channels::FutOptSubscription;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// One probe result.
///
/// - `Schema` — the SDK type could not decode a 2xx server payload. **This is
///   the landmine class** (e.g. `endSession: "1"` vs `i32`, a missing
///   non-Option field, or a bare-`Vec` decode against a wrapped object).
/// - `HttpErr` — server returned non-2xx (4xx/5xx). Not a decode bug; could be
///   a stale symbol (404) or a server-side validation reject (400).
/// - `ParamErr` — the SDK rejected the request before sending (missing
///   required param / bad config). Indicates harness misuse, not an SDK bug.
/// - `AuthErr` — bad/missing key.
/// - `NoData` — WS channel produced no snapshot within the window (neutral).
#[derive(Debug)]
enum Outcome {
    Pass,
    Schema(String),
    HttpErr(String),
    ParamErr(String),
    AuthErr(String),
    NoData,
}

impl Outcome {
    fn to_json(&self) -> Value {
        match self {
            Outcome::Pass => json!("Pass"),
            Outcome::NoData => json!("NoData"),
            Outcome::Schema(m) => json!({ "Schema": m }),
            Outcome::HttpErr(m) => json!({ "HttpErr": m }),
            Outcome::ParamErr(m) => json!({ "ParamErr": m }),
            Outcome::AuthErr(m) => json!({ "AuthErr": m }),
        }
    }
    /// Only a `Schema` mismatch is a hard SDK failure for exit-code purposes.
    /// HTTP/param/auth issues are operational, not decode landmines.
    fn is_sdk_bug(&self) -> bool {
        matches!(self, Outcome::Schema(_))
    }
}

struct Row {
    name: String,
    outcome: Outcome,
}

/// Classify by `MarketDataError` *variant*, not `source_kind()`.
///
/// `source_kind()` collapses `DeserializationError`, `ApiError(4xx)`,
/// `InvalidParameter`, and `Other` all into `ErrorKind::Client`, which makes a
/// 400 "bad param" indistinguishable from a real decode landmine. We need that
/// distinction, so we branch on the concrete variant. Note the SDK is
/// inconsistent: some decode paths return `DeserializationError`, others wrap
/// the serde error in `Other` (e.g. `Response::into_json` →
/// `Other(anyhow)`), so `Other` is sniffed for serde phrasing.
fn classify(e: &MarketDataError) -> Outcome {
    use MarketDataError as E;
    let msg = e.to_string();
    let looks_like_decode = msg.contains("invalid type")
        || msg.contains("missing field")
        || msg.contains("expected ")
        || msg.contains("Failed to read JSON")
        || msg.contains("Failed to parse");
    match e {
        E::DeserializationError { .. } => Outcome::Schema(msg),
        E::ApiError { .. } => Outcome::HttpErr(msg),
        E::AuthError { .. } => Outcome::AuthErr(msg),
        E::InvalidParameter { .. } | E::InvalidSymbol { .. } | E::ConfigError(_) => {
            Outcome::ParamErr(msg)
        }
        E::Other(_) if looks_like_decode => Outcome::Schema(msg),
        _ => Outcome::HttpErr(msg),
    }
}

fn spawn_rest<F>(name: &'static str, rest: Arc<RestClient>, f: F) -> JoinHandle<Row>
where
    F: FnOnce(&RestClient) -> Result<(), MarketDataError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let outcome = match f(&rest) {
            Ok(()) => Outcome::Pass,
            Err(e) => classify(&e),
        };
        Row { name: name.to_string(), outcome }
    })
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let key = std::env::var("FUGLE_API_KEY").expect("FUGLE_API_KEY environment variable not set");
    let rest = Arc::new(RestClient::new(Auth::ApiKey(key.clone())));

    // === REST sweep: every call on the tokio blocking pool, all in flight at
    // once. ureq is synchronous, so spawn_blocking is what gives overlap. ===
    let mut handles: Vec<JoinHandle<Row>> = Vec::new();
    macro_rules! rest_probe {
        ($name:literal, $body:expr) => {
            handles.push(spawn_rest($name, rest.clone(), |c| $body(c).map(|_| ())));
        };
    }

    // Stock intraday (6)
    rest_probe!("rest stock/intraday/quote 2330", |c: &RestClient| c
        .stock().intraday().quote().symbol("2330").send());
    rest_probe!("rest stock/intraday/ticker 2330", |c: &RestClient| c
        .stock().intraday().ticker().symbol("2330").send());
    rest_probe!("rest stock/intraday/tickers EQUITY", |c: &RestClient| c
        .stock().intraday().tickers().typ("EQUITY").send());
    rest_probe!("rest stock/intraday/trades 2330", |c: &RestClient| c
        .stock().intraday().trades().symbol("2330").send());
    rest_probe!("rest stock/intraday/volumes 2330", |c: &RestClient| c
        .stock().intraday().volumes().symbol("2330").send());
    rest_probe!("rest stock/intraday/candles 2330", |c: &RestClient| c
        .stock().intraday().candles().symbol("2330").send());

    // Stock snapshot (3)
    rest_probe!("rest stock/snapshot/quotes TSE", |c: &RestClient| c
        .stock().snapshot().quotes().market("TSE").send());
    rest_probe!("rest stock/snapshot/movers TSE", |c: &RestClient| c
        .stock().snapshot().movers().market("TSE").direction("up").change("percent").send());
    rest_probe!("rest stock/snapshot/actives TSE", |c: &RestClient| c
        .stock().snapshot().actives().market("TSE").trade("volume").send());

    // Stock ownership (1) — new in 0.8.0. 0050 is the largest, longest-lived
    // ETF, so it always has a holdings series to decode.
    rest_probe!("rest stock/ownership/etf-holdings 0050", |c: &RestClient| c
        .stock().ownership().etf_holdings().symbol("0050").send());

    // Stock historical (2) — StatsResponse is all-non-Option, watch closely
    rest_probe!("rest stock/historical/candles 2330", |c: &RestClient| c
        .stock().historical().candles().symbol("2330").send());
    rest_probe!("rest stock/historical/stats 2330", |c: &RestClient| c
        .stock().historical().stats().symbol("2330").send());

    // Stock technical (5) — every response has non-Option data_type: String.
    // Required indicator params supplied so a 400 means a real schema/usage
    // issue, not a missing-param harness defect.
    rest_probe!("rest stock/technical/sma 2330", |c: &RestClient| c
        .stock().technical().sma().symbol("2330").period(20).send());
    rest_probe!("rest stock/technical/rsi 2330", |c: &RestClient| c
        .stock().technical().rsi().symbol("2330").period(14).send());
    // kdj: prod requires rPeriod/kPeriod/dPeriod (0.7.2 added setters).
    rest_probe!("rest stock/technical/kdj 2330", |c: &RestClient| c
        .stock().technical().kdj().symbol("2330")
        .r_period(9).k_period(3).d_period(3).send());
    rest_probe!("rest stock/technical/macd 2330", |c: &RestClient| c
        .stock().technical().macd().symbol("2330").fast(12).slow(26).signal(9).send());
    rest_probe!("rest stock/technical/bb 2330", |c: &RestClient| c
        .stock().technical().bb().symbol("2330").period(20).stddev(2.0).send());

    // FutOpt intraday (7) — products is the known-broken endSession landmine
    rest_probe!("rest futopt/intraday/products FUTURE", |c: &RestClient| c
        .futopt().intraday().products().typ(FutOptType::Future).send());
    rest_probe!("rest futopt/intraday/tickers FUTURE", |c: &RestClient| c
        .futopt().intraday().tickers().typ(FutOptType::Future).send());
    // isSpread filter, new in 0.8.0.
    rest_probe!("rest futopt/intraday/tickers FUTURE isSpread", |c: &RestClient| c
        .futopt().intraday().tickers().typ(FutOptType::Future)
        .after_hours().is_spread(true).send());

    // Resolve a live futures contract off-thread. The plain-session tickers
    // list is empty at most times; the AFTERHOURS list is populated, and a
    // TXF contract code (e.g. TXFF6) is valid on the regular-session quote/
    // candles/etc endpoints too. Prefer a discovered TXF* symbol; fall back to
    // the current near-month. PROBE, not a GATE — the sweep runs regardless.
    let futopt_symbol = {
        let r = rest.clone();
        tokio::task::spawn_blocking(move || {
            r.futopt()
                .intraday()
                .tickers()
                .typ(FutOptType::Future)
                .after_hours()
                .send()
                .ok()
                .and_then(|v| {
                    v.iter()
                        .map(|t| t.symbol.clone())
                        .find(|s| s.starts_with("TXF"))
                        .or_else(|| v.into_iter().next().map(|t| t.symbol))
                })
        })
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "TXFF6".to_string()) // near-month TXF as of 2026-05-16
    };

    // Resolve a live SPREAD contract (價差). Its symbol carries a `/`
    // (e.g. "TXFF6/TXFG6"), which is exactly what the 0.8.0 percent-encoding
    // exists for: unescaped, the slash becomes a path separator and the
    // request silently lands on a different endpoint. PROBE, not a GATE —
    // if no spread contract is listed right now, the probe is skipped.
    let spread_symbol = {
        let r = rest.clone();
        tokio::task::spawn_blocking(move || {
            r.futopt()
                .intraday()
                .tickers()
                .typ(FutOptType::Future)
                .after_hours()
                .is_spread(true)
                .send()
                .ok()
                .and_then(|v| v.into_iter().map(|t| t.symbol).find(|s| s.contains('/')))
        })
        .await
        .ok()
        .flatten()
    };

    match &spread_symbol {
        Some(sym) => {
            let s = sym.clone();
            rest_probe!("rest futopt/intraday/quote SPREAD", move |c: &RestClient| c
                .futopt().intraday().quote().symbol(&s).send());
            let s = sym.clone();
            rest_probe!("rest futopt/intraday/trades SPREAD", move |c: &RestClient| c
                .futopt().intraday().trades().symbol(&s).send());
        }
        None => eprintln!(
            "note: no spread contract listed — skipping the SPREAD probes. \
             Percent-encoding of `/` in symbols is left unverified this run."
        ),
    }

    let s = futopt_symbol.clone();
    rest_probe!("rest futopt/intraday/quote", move |c: &RestClient| c
        .futopt().intraday().quote().symbol(&s).send());
    let s = futopt_symbol.clone();
    rest_probe!("rest futopt/intraday/ticker", move |c: &RestClient| c
        .futopt().intraday().ticker().symbol(&s).send());
    let s = futopt_symbol.clone();
    rest_probe!("rest futopt/intraday/candles", move |c: &RestClient| c
        .futopt().intraday().candles().symbol(&s).send());
    let s = futopt_symbol.clone();
    rest_probe!("rest futopt/intraday/trades", move |c: &RestClient| c
        .futopt().intraday().trades().symbol(&s).send());
    let s = futopt_symbol.clone();
    rest_probe!("rest futopt/intraday/volumes", move |c: &RestClient| c
        .futopt().intraday().volumes().symbol(&s).send());

    // FutOpt historical candles: uses the *continuous* product code (e.g.
    // "TXF"), NOT the month contract ("TXFF6") that intraday wants — derive
    // it by stripping the 2-char month/year suffix.
    let hist_sym = if futopt_symbol.len() > 2 {
        futopt_symbol[..futopt_symbol.len() - 2].to_string()
    } else {
        futopt_symbol.clone()
    };
    rest_probe!("rest futopt/historical/candles", move |c: &RestClient| c
        .futopt().historical().candles().symbol(&hist_sym).send());
    // futopt/historical/daily intentionally NOT probed: endpoint is not
    // provided by the live API (always HTTP 404) — deprecated in 0.7.3.

    let mut rows: Vec<Row> = Vec::new();
    for h in handles {
        rows.push(h.await.expect("rest probe task panicked"));
    }

    // === WS sweep: stock + futopt connections opened concurrently. ===
    let (mut stock_rows, mut futopt_rows) = tokio::join!(
        smoke_ws_stock(&key, "2330", "IX0001"),
        smoke_ws_futopt(&key, &futopt_symbol),
    );
    rows.append(&mut stock_rows);
    rows.append(&mut futopt_rows);

    // === Report: one JSON object per line, grep/jq friendly. ===
    let mut any_sdk_bug = false;
    for row in &rows {
        any_sdk_bug |= row.outcome.is_sdk_bug();
        println!(
            "{}",
            json!({ "name": row.name, "outcome": row.outcome.to_json() })
        );
    }

    if any_sdk_bug {
        std::process::exit(1);
    }
}

/// Subscribe every stock channel on one connection, then drain frames until
/// each channel has yielded one snapshot/data frame (force-parsed via the
/// public `parse_channel_data`) or the global deadline passes.
async fn smoke_ws_stock(key: &str, equity: &str, index: &str) -> Vec<Row> {
    let want: &[(Channel, &str, &str)] = &[
        (Channel::Trades, equity, "ws stock trades"),
        (Channel::Candles, equity, "ws stock candles"),
        (Channel::Books, equity, "ws stock books"),
        (Channel::Aggregates, equity, "ws stock aggregates"),
        (Channel::Indices, index, "ws stock indices"),
    ];
    let cfg = ConnectionConfig::fugle_stock(AuthRequest::with_api_key(key));
    let client = AsyncWs::new(cfg);
    let mut rx = client.message_stream();

    if let Err(e) = client.connect().await {
        return vec![Row { name: "ws stock connect".into(), outcome: classify(&e) }];
    }
    for (ch, sym, _) in want {
        let _ = client.subscribe(StockSubscription::new(ch.clone(), *sym)).await;
    }

    let labels: BTreeMap<&str, &str> = want
        .iter()
        .map(|(ch, _, label)| (ch.as_str(), *label))
        .collect();
    let rows = drain_snapshots(&mut rx, &labels).await;

    let _ = client.shutdown_with_timeout(Duration::from_secs(5)).await;
    rows
}

/// FutOpt counterpart. Channels: trades, books, candles, aggregates (no
/// indices on the futopt feed).
async fn smoke_ws_futopt(key: &str, symbol: &str) -> Vec<Row> {
    let want: &[(FutOptChannel, &str)] = &[
        (FutOptChannel::Trades, "ws futopt trades"),
        (FutOptChannel::Books, "ws futopt books"),
        (FutOptChannel::Candles, "ws futopt candles"),
        (FutOptChannel::Aggregates, "ws futopt aggregates"),
    ];
    // Since 0.8.0 this resolves to streaming v1.1, so the frames drained here
    // may carry `isTrial` / `derivedBid` / `derivedAsk`. A decode failure on
    // those is exactly what this sweep is meant to catch.
    let cfg = ConnectionConfig::fugle_futopt(AuthRequest::with_api_key(key));
    let client = AsyncWs::new(cfg);
    let mut rx = client.message_stream();

    if let Err(e) = client.connect().await {
        return vec![Row { name: "ws futopt connect".into(), outcome: classify(&e) }];
    }
    for (ch, _) in want {
        let _ = client
            .subscribe_futopt(FutOptSubscription::new(ch.clone(), symbol))
            .await;
    }

    let labels: BTreeMap<&str, &str> =
        want.iter().map(|(ch, label)| (ch.as_str(), *label)).collect();
    let rows = drain_snapshots(&mut rx, &labels).await;

    let _ = client.shutdown_with_timeout(Duration::from_secs(5)).await;
    rows
}

/// Drain the message stream, force-parsing the first snapshot/data frame seen
/// per channel. Channel name → human label. Any channel that produces no
/// frame before the deadline is reported `NoData` (neutral). Global deadline
/// is generous (10 s) because snapshots normally arrive <200 ms after the
/// subscribe ACK; the budget only matters for genuinely quiet channels.
async fn drain_snapshots(
    rx: &mut tokio::sync::mpsc::Receiver<marketdata_core::WebSocketMessage>,
    labels: &BTreeMap<&str, &str>,
) -> Vec<Row> {
    let mut seen: BTreeMap<&str, Outcome> = BTreeMap::new();
    let deadline = Instant::now() + Duration::from_secs(10);

    while seen.len() < labels.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let msg = match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(m)) => m,
            Ok(None) | Err(_) => break, // stream closed or global deadline
        };

        if msg.event != "snapshot" && msg.event != "data" {
            continue; // authenticated / subscribed / pong / heartbeat / error
        }
        let Some(channel) = msg.channel.as_deref() else {
            continue;
        };
        if !labels.contains_key(channel) || seen.contains_key(channel) {
            continue;
        }
        let Some(data) = msg.data.as_ref() else {
            continue;
        };
        let is_snapshot = msg.event == "snapshot";
        let outcome = match parse_channel_data(channel, data, is_snapshot) {
            Ok(_) => Outcome::Pass,
            Err(e) => classify(&e),
        };
        // BTreeMap key must outlive the loop; labels keys are borrowed from
        // the channel as_str() literals, so re-borrow from labels.
        let key = *labels.get_key_value(channel).expect("present").0;
        seen.insert(key, outcome);
    }

    labels
        .iter()
        .map(|(ch, label)| Row {
            name: (*label).to_string(),
            outcome: seen.remove(*ch).unwrap_or(Outcome::NoData),
        })
        .collect()
}
