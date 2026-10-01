//! Per-product WebSocket streaming versions.
//!
//! Each product serves its own set of streaming versions, and the sets are not
//! the same. The official Node / Python SDKs express this as a per-product map
//! (`{ futopt: 'v1.1' }`) validated at runtime, because a bare version string
//! would be ambiguous about which product it applied to.
//!
//! Rust doesn't need the runtime check: a distinct enum per product makes an
//! unsupported pairing unrepresentable, so `stock` can never be asked for a
//! version it doesn't serve.
//!
//! ```compile_fail
//! use marketdata_core::websocket::{FutOptVersion, WebSocketFactory};
//! use marketdata_core::AuthRequest;
//!
//! // `FutOptVersion` is not a `StockVersion` — futopt-only versions cannot
//! // reach the stock endpoint.
//! let _ = WebSocketFactory::new()
//!     .auth(AuthRequest::with_api_key("k"))
//!     .stock_version(FutOptVersion::V1_1);
//! ```

/// Appended to `base_url` rejections, pointing at the options that own the
/// version segment for streaming.
pub(crate) const VERSION_OPTION_HINT: &str =
    "The version comes from the streaming version options, \
     e.g. .futopt_version(FutOptVersion::V1_1).";

/// Streaming versions served by the stock product.
///
/// Stock has no v1.1: its trial-matching (試撮) frames have always been
/// streamed, so there was no compatibility break to gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum StockVersion {
    /// `v1.0` — the only version stock streaming serves.
    #[default]
    V1_0,
}

impl StockVersion {
    /// The URL path segment for this version.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1_0 => "v1.0",
        }
    }
}

/// Streaming versions served by the futures/options product.
///
/// `V1_1` is `V1_0` plus trial-matching (試撮, TAIFEX I022/I082) frames on the
/// `trades` and `books` channels, which carry a top-level `isTrial: true`.
///
/// Trial frames on `trades` / `books` only reach clients connected to v1.1.
/// They are *not* the only place trial data surfaces, though: `aggregates` is
/// not version-gated, and during a trial session its `lastPrice` / `lastSize`
/// are the trial values on every version — only the top-level `isTrial`
/// distinguishes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FutOptVersion {
    /// `v1.0` — no trial-matching frames on `trades` / `books`.
    V1_0,
    /// `v1.1` — adds trial-matching frames. **This is the default.**
    #[default]
    V1_1,
}

impl FutOptVersion {
    /// The URL path segment for this version.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1_0 => "v1.0",
            Self::V1_1 => "v1.1",
        }
    }
}

/// The bindings' `version` option (`{ futopt: 'v1.0' }` / `{"futopt": "v1.0"}`)
/// resolved against the versions each product serves (#294).
///
/// Hidden from the docs and the public-API baseline like
/// [`rest::params`](crate::rest::params): it serves the bindings, which only
/// add the shape checks and the wording of their own map syntax.
#[doc(hidden)]
pub mod option {
    use super::{FutOptVersion, StockVersion};

    /// Product names as the option spells them, with the versions each one
    /// serves, oldest first: the last entry is the product's default.
    pub const PRODUCTS: [(&str, &[&str]); 2] = [("stock", &["v1.0"]), ("futopt", &["v1.0", "v1.1"])];

    /// Why one entry of the option was refused.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum VersionOptionError {
        /// A key that is not a product name.
        UnknownProduct(String),
        /// A version the product does not serve.
        Unsupported {
            product: &'static str,
            requested: String,
            supported: &'static [&'static str],
        },
    }

    impl VersionOptionError {
        /// The binding-neutral sentence: `futopt streaming does not support
        /// v9 (supported: v1.0, v1.1).` The binding appends how to fix it in
        /// its own syntax ([`default_for`] names the version omitting it gives).
        pub fn describe(&self) -> String {
            match self {
                Self::UnknownProduct(key) => {
                    format!("unknown product '{key}' (known: {})", product_names().join(", "))
                }
                Self::Unsupported { product, requested, supported } => format!(
                    "{product} streaming does not support {requested} (supported: {}).",
                    supported.join(", ")
                ),
            }
        }
    }

    /// Product names in the order of [`PRODUCTS`].
    pub fn product_names() -> Vec<&'static str> {
        PRODUCTS.iter().map(|(name, _)| *name).collect()
    }

    /// The version a product gets when the option leaves it out.
    pub fn default_for(product: &str) -> Option<&'static str> {
        PRODUCTS.iter().find(|(name, _)| *name == product).and_then(|(_, v)| v.last().copied())
    }

    /// Products that serve `version`, for the hint on a bare version string:
    /// `"v1.0"` is `["stock", "futopt"]`, `"v1.1"` is `["futopt"]`.
    pub fn products_serving(version: &str) -> Vec<&'static str> {
        PRODUCTS
            .iter()
            .filter(|(_, versions)| versions.contains(&version))
            .map(|(name, _)| *name)
            .collect()
    }

    /// Resolve `(product, version)` entries; products left out keep their
    /// default. The first bad entry is the error.
    pub fn resolve<'a>(
        entries: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<(StockVersion, FutOptVersion), VersionOptionError> {
        let mut stock = StockVersion::default();
        let mut futopt = FutOptVersion::default();
        for (product, requested) in entries {
            let unsupported = |product: &'static str| {
                let supported = PRODUCTS.iter().find(|(name, _)| *name == product).map(|(_, v)| *v).unwrap_or(&[]);
                VersionOptionError::Unsupported { product, requested: requested.to_string(), supported }
            };
            match product {
                "stock" => {
                    stock = match requested {
                        "v1.0" => StockVersion::V1_0,
                        _ => return Err(unsupported("stock")),
                    }
                }
                "futopt" => {
                    futopt = match requested {
                        "v1.0" => FutOptVersion::V1_0,
                        "v1.1" => FutOptVersion::V1_1,
                        _ => return Err(unsupported("futopt")),
                    }
                }
                other => return Err(VersionOptionError::UnknownProduct(other.to_string())),
            }
        }
        Ok((stock, futopt))
    }
}

#[cfg(test)]
mod tests {
    use super::option::{default_for, products_serving, resolve, VersionOptionError, PRODUCTS};
    use super::*;

    #[test]
    fn test_option_table_matches_enums() {
        assert_eq!(default_for("stock"), Some(StockVersion::default().as_str()));
        assert_eq!(default_for("futopt"), Some(FutOptVersion::default().as_str()));
        for (product, versions) in PRODUCTS {
            for v in versions {
                assert!(resolve([(product, *v)]).is_ok(), "{product} {v}");
            }
        }
    }

    #[test]
    fn test_option_refuses_versions_outside_the_table() {
        for (product, versions) in PRODUCTS {
            for v in ["v0.9", "v1.2", "v2.0", "1.0", "V1.0", ""] {
                assert!(!versions.contains(&v));
                assert!(resolve([(product, v)]).is_err(), "{product} {v:?}");
            }
        }
        assert!(resolve([("stock", "v1.1")]).is_err(), "futopt-only version on stock");
    }

    #[test]
    fn test_option_resolve() {
        assert_eq!(resolve([]), Ok((StockVersion::V1_0, FutOptVersion::V1_1)));
        assert_eq!(resolve([("futopt", "v1.0")]), Ok((StockVersion::V1_0, FutOptVersion::V1_0)));
        let err = resolve([("futopt", "v9")]).unwrap_err();
        assert_eq!(err.describe(), "futopt streaming does not support v9 (supported: v1.0, v1.1).");
        assert_eq!(
            resolve([("stock", "v1.1")]).unwrap_err().describe(),
            "stock streaming does not support v1.1 (supported: v1.0)."
        );
        assert_eq!(resolve([("foo", "v1.0")]), Err(VersionOptionError::UnknownProduct("foo".into())));
        assert_eq!(
            VersionOptionError::UnknownProduct("foo".into()).describe(),
            "unknown product 'foo' (known: stock, futopt)"
        );
    }

    #[test]
    fn test_products_serving() {
        assert_eq!(products_serving("v1.0"), ["stock", "futopt"]);
        assert_eq!(products_serving("v1.1"), ["futopt"]);
        assert!(products_serving("v9").is_empty());
    }

    #[test]
    fn test_stock_default_is_v1_0() {
        assert_eq!(StockVersion::default(), StockVersion::V1_0);
        assert_eq!(StockVersion::default().as_str(), "v1.0");
    }

    #[test]
    fn test_futopt_default_is_v1_1() {
        // Behaviour change in 0.8.0: futopt defaults to the latest version,
        // which means trial frames arrive without opting in.
        assert_eq!(FutOptVersion::default(), FutOptVersion::V1_1);
        assert_eq!(FutOptVersion::default().as_str(), "v1.1");
    }

    #[test]
    fn test_futopt_v1_0_still_reachable() {
        assert_eq!(FutOptVersion::V1_0.as_str(), "v1.0");
    }
}
