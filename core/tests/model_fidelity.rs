//! Guards the optional typed models against silently losing fields.
//!
//! REST responses reach callers as raw JSON, so these models are no longer on
//! the return path — they exist for Rust callers who want static types via
//! `serde_json::from_value`. That makes them easy to forget about, which is
//! how `Quote` came to be missing `referencePrice` and `serial` for as long as
//! it did: nothing failed, the fields just vanished.
//!
//! Each test here decodes a real recorded response into its model, serializes
//! it back, and asserts that every key the server sent survived the round
//! trip. Extra keys in the output are tolerated (absent optionals serialize as
//! `null`); missing ones are not.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Deliberate, documented departures from the wire format.
///
/// Anything listed here is a decision someone made on purpose; everything else
/// is a bug. Keep this list short and keep the reasons with it.
fn is_known_deviation(path: &str) -> bool {
    // `serial` is normalised to a string. The server types it as a number for
    // stock and as a zero-padded string for futopt (`"00379320"`), and parsing
    // the latter as a number would drop the leading zeros. See
    // `deserialize_serial` in core/src/models/common.rs.
    path.ends_with(".serial")
}

/// Assert that every key present in `original` is still present in `round_tripped`,
/// with an equal value. Reports the full JSON path of the first loss found.
fn assert_no_field_lost(original: &Value, round_tripped: &Value, path: &str) {
    if is_known_deviation(path) {
        return;
    }
    match (original, round_tripped) {
        (Value::Object(orig), Value::Object(back)) => {
            for (key, orig_val) in orig {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                match back.get(key) {
                    None => panic!(
                        "field `{}` was dropped by the model — the server sent it, \
                         the struct has no matching field",
                        child
                    ),
                    Some(back_val) => assert_no_field_lost(orig_val, back_val, &child),
                }
            }
        }
        (Value::Array(orig), Value::Array(back)) => {
            assert_eq!(
                orig.len(),
                back.len(),
                "array length changed at `{}`",
                path
            );
            for (i, (o, b)) in orig.iter().zip(back).enumerate() {
                assert_no_field_lost(o, b, &format!("{}[{}]", path, i));
            }
        }
        // Numbers are compared by value, not by JSON representation. A model
        // that types a price as `f64` turns the server's `2380` into `2380.0`;
        // that is a representation difference, not a lost or altered value.
        // (Callers on the default path never see it — raw passthrough keeps
        // the server's own representation.)
        (Value::Number(o), Value::Number(b)) => assert_eq!(
            o.as_f64(),
            b.as_f64(),
            "numeric value changed at `{}`",
            path
        ),
        (o, b) => assert_eq!(o, b, "value changed at `{}`", path),
    }
}

/// Decode `raw` into `T`, serialize it back, and assert nothing was lost.
fn assert_model_is_lossless<T: DeserializeOwned + Serialize>(raw: &str) {
    let original: Value = serde_json::from_str(raw).expect("fixture is not valid JSON");
    let decoded: T = serde_json::from_str(raw).expect("model failed to decode the response");
    let round_tripped = serde_json::to_value(&decoded).expect("model failed to serialize");
    assert_no_field_lost(&original, &round_tripped, "");
}

/// A real `GET /stock/intraday/quote/2330` body, captured 2026-09-16.
///
/// This exact response is what surfaced the bug: `referencePrice` and the
/// top-level `serial` were absent from `Quote`, so callers could not reach
/// them. `referencePrice` matters more than it looks — note that
/// `lastPrice - referencePrice == change` (2385 - 2380 == 5) while
/// `previousClose` is 2385, so anyone deriving the move from `previousClose`
/// gets 0 instead of 5.
const STOCK_QUOTE_2330: &str = r#"{
    "date": "2026-09-16",
    "type": "EQUITY",
    "exchange": "TWSE",
    "market": "TSE",
    "symbol": "2330",
    "name": "台積電",
    "referencePrice": 2380,
    "previousClose": 2385,
    "openPrice": 2375,
    "openTime": 1789520409721819,
    "highPrice": 2385,
    "highTime": 1789522040749587,
    "lowPrice": 2375,
    "lowTime": 1789520409721819,
    "closePrice": 2385,
    "closeTime": 1789525853959140,
    "avgPrice": 2378.51,
    "change": 5,
    "changePercent": 0.21,
    "amplitude": 0.42,
    "lastPrice": 2385,
    "lastSize": 1,
    "bids": [
        { "price": 2380, "size": 185 },
        { "price": 2375, "size": 1407 },
        { "price": 2370, "size": 1248 },
        { "price": 2365, "size": 672 },
        { "price": 2360, "size": 810 }
    ],
    "asks": [
        { "price": 2385, "size": 199 },
        { "price": 2390, "size": 556 },
        { "price": 2395, "size": 305 },
        { "price": 2400, "size": 488 },
        { "price": 2405, "size": 233 }
    ],
    "total": {
        "tradeValue": 14566025000,
        "tradeVolume": 6124,
        "tradeVolumeAtBid": 1962,
        "tradeVolumeAtAsk": 2686,
        "transaction": 1922,
        "time": 1789525853959140
    },
    "lastTrade": {
        "bid": 2380,
        "ask": 2385,
        "price": 2385,
        "size": 1,
        "time": 1789525853959140,
        "serial": 6132837
    },
    "lastTrial": {
        "bid": 2375,
        "ask": 2380,
        "price": 2380,
        "size": 1412,
        "time": 1789520395272982,
        "serial": 87965
    },
    "isContinuous": true,
    "serial": 6152257,
    "lastUpdated": 1789525882301247
}"#;

#[test]
fn stock_quote_model_keeps_every_field_the_server_sent() {
    assert_model_is_lossless::<marketdata_core::models::Quote>(STOCK_QUOTE_2330);
}

#[test]
fn stock_quote_reference_price_is_the_basis_for_change() {
    let q: marketdata_core::models::Quote = serde_json::from_str(STOCK_QUOTE_2330).unwrap();

    let reference = q.reference_price.expect("referencePrice must decode");
    let last = q.last_price.expect("lastPrice must decode");
    let change = q.change.expect("change must decode");

    assert_eq!(last - reference, change);
    // …and deriving it from previousClose would have given the wrong answer.
    assert_ne!(last - q.previous_close.unwrap(), change);
}

#[test]
fn stock_quote_carries_its_own_serial() {
    let q: marketdata_core::models::Quote = serde_json::from_str(STOCK_QUOTE_2330).unwrap();

    // The quote's own sequence number, distinct from the last trade's.
    assert_eq!(q.serial, Some(6152257));
    assert_eq!(q.last_trade.unwrap().serial.as_deref(), Some("6132837"));
}

#[test]
fn field_loss_is_actually_detected() {
    // Proves the guard above fails when a field goes missing, rather than
    // passing vacuously the way the old fixture-only tests did.
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Incomplete {
        symbol: String,
    }

    let result = std::panic::catch_unwind(|| {
        assert_model_is_lossless::<Incomplete>(r#"{"symbol":"2330","referencePrice":2380}"#);
    });
    assert!(result.is_err(), "a dropped field must fail the guard");
}
