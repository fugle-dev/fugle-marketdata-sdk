//! [`Symbols`] — flexible single-or-many input for subscription targets.
//!
//! `impl Into<Symbols>` is implemented for every reasonable input shape so
//! the public `subscribe(channel, symbols)` API can accept a `&str`,
//! `String`, `Vec<String>`, array literal `["A", "B"]`, or slice without
//! forcing the caller to disambiguate.
//!
//! Wire form depends on the variant:
//!
//! - `Single(s)`   serializes as `{"symbol": "s"}`
//! - `Many(vec)`   serializes as `{"symbols": ["s1", "s2", ...]}`
//!
//! The Fugle server gateway natively handles both forms (see
//! `stock.gateway.ts:13`), returning a single ACK object for `Single` and
//! an ACK array for `Many`. A `Many` batch is therefore a real 1-frame-in
//! / 1-ACK-out round-trip, not an N-frame loop.
//!
//! # Normalization & deduplication
//!
//! Use [`Symbols::normalized`] to trim whitespace, drop empty entries, and
//! deduplicate while preserving insertion order. The subscription dispatch
//! path (`SubscribeRequest::with_symbols` and the channel-specific
//! `*Subscription::new`) runs `.normalized()` internally so duplicate
//! symbols collapse to one subscription before reaching
//! `SubscriptionManager`.
//!
//! # Case-sensitivity policy
//!
//! Dedup is **byte-for-byte case-sensitive**. `"TXFB6"`, `"txfb6"`, and
//! `"TxFb6"` are three distinct subscription targets. This matches the
//! TWSE / Fugle wire-format contract: lowercase and uppercase contract
//! identifiers are different on the server side. If your caller wants
//! case folding (e.g. to normalize user input from a web form), apply
//! `.into_iter().map(str::to_ascii_uppercase).collect()` **before**
//! passing the value to a constructor; do not rely on `Symbols::normalized`
//! to do it.

use indexmap::IndexSet;

/// A symbol specification: either a single symbol or a batch.
///
/// Renamed from the legacy `SymbolSpec` in 0.5.0. See the module-level
/// docs for the wire-format contract and normalization semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbols {
    /// Single-symbol subscription.
    Single(String),
    /// Batch subscription. Insertion order is preserved.
    Many(Vec<String>),
}

impl Symbols {
    /// Trim leading/trailing whitespace on each entry, drop empties
    /// (post-trim), deduplicate **byte-for-byte case-sensitive** preserving
    /// first-seen order, and collapse `Many` of length 1 down to `Single`
    /// for canonical form.
    ///
    /// Empty `Many` (after dropping empties) collapses to `Many(vec![])`
    /// rather than panicking — the calling site decides how to handle it.
    #[must_use]
    pub fn normalized(self) -> Self {
        match self {
            Symbols::Single(s) => {
                let t = s.trim();
                if t.is_empty() {
                    Symbols::Many(Vec::new())
                } else {
                    Symbols::Single(t.to_owned())
                }
            }
            Symbols::Many(v) => {
                let mut dedup = IndexSet::with_capacity(v.len());
                for s in v {
                    let t = s.trim();
                    if !t.is_empty() {
                        dedup.insert(t.to_owned());
                    }
                }
                let mut out: Vec<String> = dedup.into_iter().collect();
                if out.len() == 1 {
                    Symbols::Single(out.pop().expect("len == 1"))
                } else {
                    Symbols::Many(out)
                }
            }
        }
    }

    /// Number of symbols carried.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Symbols::Single(_) => 1,
            Symbols::Many(v) => v.len(),
        }
    }

    /// True when no symbols are carried.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Symbols::Single(_) => false,
            Symbols::Many(v) => v.is_empty(),
        }
    }

    /// Iterate symbols in insertion order.
    pub fn iter(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self {
            Symbols::Single(s) => Box::new(std::iter::once(s.as_str())),
            Symbols::Many(v) => Box::new(v.iter().map(String::as_str)),
        }
    }

    /// Split into chunks of at most `max_per_chunk` symbols each, preserving
    /// insertion order. Chunks of length 1 are returned as `Single` for
    /// canonical form.
    ///
    /// # Panics
    /// Panics if `max_per_chunk == 0`.
    #[must_use]
    pub fn chunked(self, max_per_chunk: usize) -> Vec<Symbols> {
        assert!(max_per_chunk > 0, "max_per_chunk must be non-zero");
        let items: Vec<String> = match self {
            Symbols::Single(s) => vec![s],
            Symbols::Many(v) => v,
        };
        if items.is_empty() {
            return Vec::new();
        }
        items
            .chunks(max_per_chunk)
            .map(|chunk| {
                if chunk.len() == 1 {
                    Symbols::Single(chunk[0].clone())
                } else {
                    Symbols::Many(chunk.to_vec())
                }
            })
            .collect()
    }
}

/// Future-proof reservation for a per-frame symbol limit advertised by the
/// Fugle server gateway. Currently `None`: the dispatch path emits a single
/// subscription frame regardless of symbol count. When/if the gateway
/// publishes a per-frame limit, the dispatch path will adopt
/// [`Symbols::chunked`] with this constant as the chunk size.
pub const SUBSCRIPTION_BATCH_LIMIT: Option<usize> = None;

// ----- From conversions -----

impl From<&str> for Symbols {
    fn from(s: &str) -> Self {
        Symbols::Single(s.to_owned())
    }
}

impl From<String> for Symbols {
    fn from(s: String) -> Self {
        Symbols::Single(s)
    }
}

impl From<&String> for Symbols {
    fn from(s: &String) -> Self {
        Symbols::Single(s.clone())
    }
}

impl From<Vec<String>> for Symbols {
    fn from(v: Vec<String>) -> Self {
        Symbols::Many(v)
    }
}

impl From<Vec<&str>> for Symbols {
    fn from(v: Vec<&str>) -> Self {
        Symbols::Many(v.into_iter().map(String::from).collect())
    }
}

impl<const N: usize> From<[&str; N]> for Symbols {
    fn from(arr: [&str; N]) -> Self {
        Symbols::Many(arr.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl<const N: usize> From<[String; N]> for Symbols {
    fn from(arr: [String; N]) -> Self {
        Symbols::Many(arr.into_iter().collect())
    }
}

impl From<&[&str]> for Symbols {
    fn from(s: &[&str]) -> Self {
        Symbols::Many(s.iter().map(|s| (*s).to_owned()).collect())
    }
}

impl From<&[String]> for Symbols {
    fn from(s: &[String]) -> Self {
        Symbols::Many(s.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_produces_single() {
        assert_eq!(Symbols::from("2330"), Symbols::Single("2330".to_string()));
    }

    #[test]
    fn from_vec_produces_many() {
        assert_eq!(
            Symbols::from(vec!["2330", "2454"]),
            Symbols::Many(vec!["2330".into(), "2454".into()])
        );
    }

    #[test]
    fn normalize_preserves_case() {
        // Three entries with different cases must all be retained — byte-
        // for-byte dedup, no case folding. The TWSE / Fugle wire contract
        // treats lowercase and uppercase identifiers as distinct subscriptions.
        let s = Symbols::from(vec!["TXFB6", "txfb6", "TxFb6"]).normalized();
        assert_eq!(
            s,
            Symbols::Many(vec!["TXFB6".into(), "txfb6".into(), "TxFb6".into()])
        );
    }

    #[test]
    fn normalize_preserves_case_after_whitespace_trim() {
        // Whitespace trim runs FIRST, then dedup. After trimming, "TXFB6"
        // and "txfb6" are still case-distinct and both retained.
        let s = Symbols::from(vec!["  TXFB6 ", "txfb6"]).normalized();
        assert_eq!(s, Symbols::Many(vec!["TXFB6".into(), "txfb6".into()]));
    }

    #[test]
    fn normalize_trims_whitespace() {
        let s = Symbols::from(vec!["  2330  ", "2454\n"]).normalized();
        assert_eq!(s, Symbols::Many(vec!["2330".into(), "2454".into()]));
    }

    #[test]
    fn normalize_dedup_preserves_order() {
        let s = Symbols::from(vec!["2330", "2454", "2330", "2317"]).normalized();
        assert_eq!(
            s,
            Symbols::Many(vec!["2330".into(), "2454".into(), "2317".into()])
        );
    }

    #[test]
    fn normalize_drops_empties() {
        let s = Symbols::from(vec!["2330", "", "  ", "2454"]).normalized();
        assert_eq!(s, Symbols::Many(vec!["2330".into(), "2454".into()]));
    }

    #[test]
    fn normalize_collapses_many_of_one_to_single() {
        let s = Symbols::from(vec!["2330"]).normalized();
        assert_eq!(s, Symbols::Single("2330".to_string()));
    }

    #[test]
    fn normalize_single_trims() {
        let s = Symbols::Single("  2330  ".to_string()).normalized();
        assert_eq!(s, Symbols::Single("2330".to_string()));
    }

    #[test]
    fn normalize_single_empty_becomes_empty_many() {
        let s = Symbols::Single("   ".to_string()).normalized();
        assert!(s.is_empty());
    }

    #[test]
    fn len_reflects_symbol_count() {
        assert_eq!(Symbols::from("2330").len(), 1);
        assert_eq!(Symbols::from(vec!["2330", "2454"]).len(), 2);
    }

    #[test]
    fn iter_yields_in_order() {
        let s = Symbols::from(vec!["A", "B", "C"]);
        let collected: Vec<&str> = s.iter().collect();
        assert_eq!(collected, vec!["A", "B", "C"]);
    }

    #[test]
    fn chunked_splits_into_requested_sizes() {
        let chunks = Symbols::from(vec!["A", "B", "C", "D", "E"]).chunked(2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], Symbols::Many(vec!["A".into(), "B".into()]));
        assert_eq!(chunks[1], Symbols::Many(vec!["C".into(), "D".into()]));
        assert_eq!(chunks[2], Symbols::Single("E".into()));
    }

    #[test]
    #[should_panic(expected = "max_per_chunk must be non-zero")]
    fn chunked_zero_panics() {
        let _ = Symbols::from(vec!["A"]).chunked(0);
    }

    #[test]
    fn subscription_batch_limit_is_none() {
        assert_eq!(SUBSCRIPTION_BATCH_LIMIT, None);
    }
}
