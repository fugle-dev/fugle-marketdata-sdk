//! The keys of a `subscribe()` / `unsubscribe()` object (Node) or dict
//! (Python) (#294).
//!
//! Both bindings used to read the keys they knew and drop the rest, so
//! `{ channel, symbol, afterHours: true }` on the stock client subscribed
//! board-lot data without a word. Each binding now refuses any other key;
//! the keys, which product a session modifier belongs to and the message are
//! here so the two say the same thing. A binding may accept extra spellings
//! of the modifier (Python's `oddLot` / `odd_lot`); it passes its own list.
//!
//! Hidden from the docs and the public-API baseline like
//! [`rest::params`](crate::rest::params): it serves the bindings.

use super::StreamProduct;

/// The keys every subscribe-shaped object takes, before the modifier.
pub const SUBSCRIBE_KEYS: [&str; 3] = ["channel", "symbol", "symbols"];

/// The keys of the server-id form of `unsubscribe()`.
pub const ID_KEYS: [&str; 2] = ["id", "ids"];

/// The product's session modifier as the server spells it.
pub fn modifier_key(product: StreamProduct) -> &'static str {
    match product {
        StreamProduct::Stock => "intradayOddLot",
        StreamProduct::FutOpt => "afterHours",
    }
}

/// The product a modifier spelling belongs to: the server's and the
/// bindings' (`oddLot`, `odd_lot`, `after_hours`).
pub fn modifier_product(key: &str) -> Option<StreamProduct> {
    match key {
        "intradayOddLot" | "oddLot" | "odd_lot" => Some(StreamProduct::Stock),
        "afterHours" | "after_hours" => Some(StreamProduct::FutOpt),
        _ => None,
    }
}

fn product_name(product: StreamProduct) -> &'static str {
    match product {
        StreamProduct::Stock => "stock",
        StreamProduct::FutOpt => "futopt",
    }
}

/// Why `key` is refused by `product`'s object that takes `accepted`, without
/// the binding's call prefix: `unknown key 'afterHours': it is a futopt
/// option, the stock client takes intradayOddLot (accepted: channel, symbol,
/// symbols, intradayOddLot)`.
pub fn unknown_key(product: StreamProduct, key: &str, accepted: &[&str]) -> String {
    let accepted = accepted.join(", ");
    match modifier_product(key) {
        Some(owner) if owner != product => format!(
            "unknown key '{key}': it is a {} option, the {} client takes {} (accepted: {accepted})",
            product_name(owner),
            product_name(product),
            modifier_key(product),
        ),
        _ => format!("unknown key '{key}' (accepted: {accepted})"),
    }
}

/// Why `key` is refused by an `unsubscribe()` object without `channel`,
/// which names server ids: `unknown key 'symbol' (without channel the
/// object takes id or ids; to unsubscribe by channel and symbol, add channel)`.
pub fn unknown_id_key(key: &str) -> String {
    format!(
        "unknown key '{key}' (without channel the object takes {}; to unsubscribe by channel \
         and symbol, add channel)",
        ID_KEYS.join(" or ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_key_names_the_other_product() {
        let accepted = ["channel", "symbol", "symbols", "intradayOddLot"];
        assert_eq!(
            unknown_key(StreamProduct::Stock, "afterHours", &accepted),
            "unknown key 'afterHours': it is a futopt option, the stock client takes intradayOddLot \
             (accepted: channel, symbol, symbols, intradayOddLot)"
        );
        assert_eq!(
            unknown_key(StreamProduct::FutOpt, "odd_lot", &["channel"]),
            "unknown key 'odd_lot': it is a stock option, the futopt client takes afterHours (accepted: channel)"
        );
        assert_eq!(unknown_key(StreamProduct::Stock, "foo", &["channel"]), "unknown key 'foo' (accepted: channel)");
    }

    #[test]
    fn test_unknown_id_key() {
        assert_eq!(
            unknown_id_key("symbol"),
            "unknown key 'symbol' (without channel the object takes id or ids; to unsubscribe by \
             channel and symbol, add channel)"
        );
    }

    #[test]
    fn test_modifier_spellings() {
        for product in [StreamProduct::Stock, StreamProduct::FutOpt] {
            assert_eq!(modifier_product(modifier_key(product)), Some(product));
        }
    }
}
