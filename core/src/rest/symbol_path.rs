//! Percent-encoding for symbols that go into a URL path segment.
//!
//! Futures/options spread contracts carry a `/` in the symbol
//! (e.g. `TXFC4/TXFD4`). Interpolated raw, that slash becomes a path
//! separator and the request lands on a different endpoint than intended —
//! `intraday/quote/TXFC4/TXFD4` rather than a quote for the spread.
//!
//! Mirrors the official SDKs' `encodeURIComponent(symbol)` (Node) and
//! `quote(symbol, safe='')` (Python).

use std::borrow::Cow;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

/// The set `encodeURIComponent` leaves alone, expressed as what to escape.
///
/// JavaScript's `encodeURIComponent` does not escape
/// `A-Z a-z 0-9 - _ . ! ~ * ' ( )`. Starting from [`NON_ALPHANUMERIC`] and
/// removing that punctuation reproduces it exactly, so a symbol encoded here
/// and one encoded by the Node SDK produce byte-identical URLs.
const ENCODE_URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// Percent-encode `symbol` for use as a single URL path segment.
///
/// Returns a borrowed `Cow` when nothing needs escaping, which is the common
/// case — plain equity and single-leg futures symbols pass through untouched.
pub(crate) fn encode_symbol(symbol: &str) -> Cow<'_, str> {
    utf8_percent_encode(symbol, ENCODE_URI_COMPONENT).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_symbols_pass_through_unescaped() {
        for symbol in ["2330", "TXFC4", "TXO18000C4", "0050"] {
            assert_eq!(encode_symbol(symbol), symbol);
            assert!(
                matches!(encode_symbol(symbol), Cow::Borrowed(_)),
                "{symbol} needs no allocation"
            );
        }
    }

    #[test]
    fn test_spread_contract_slash_is_escaped() {
        // The reason this module exists: an unescaped `/` silently changes
        // which endpoint the request reaches.
        assert_eq!(encode_symbol("TXFC4/TXFD4"), "TXFC4%2FTXFD4");
    }

    #[test]
    fn test_matches_encode_uri_component_unreserved_set() {
        // `encodeURIComponent` leaves these alone; so must we, or a symbol
        // would encode differently here than in the Node SDK.
        let unreserved = "-_.!~*'()";
        assert_eq!(encode_symbol(unreserved), unreserved);
    }

    #[test]
    fn test_reserved_characters_are_escaped() {
        assert_eq!(encode_symbol("a b"), "a%20b");
        assert_eq!(encode_symbol("a?b"), "a%3Fb");
        assert_eq!(encode_symbol("a#b"), "a%23b");
        assert_eq!(encode_symbol("a&b"), "a%26b");
        assert_eq!(encode_symbol("a=b"), "a%3Db");
        assert_eq!(encode_symbol("a+b"), "a%2Bb");
    }

    #[test]
    fn test_already_encoded_input_is_escaped_again() {
        // Encoding is not idempotent, and must not be: a caller who passes a
        // literal `%` means a literal `%`.
        assert_eq!(encode_symbol("TXFC4%2FTXFD4"), "TXFC4%252FTXFD4");
    }

    #[test]
    fn test_non_ascii_is_utf8_percent_encoded() {
        assert_eq!(encode_symbol("臺股"), "%E8%87%BA%E8%82%A1");
    }
}
