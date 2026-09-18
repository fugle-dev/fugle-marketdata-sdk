//! Which query keys each REST endpoint accepts (#164).
//!
//! One table for every binding to check an object / kwargs call against, so a
//! typo is refused up front instead of being forwarded. The server cannot be
//! relied on for that: the gateway's `ValidationPipe` has no `whitelist`, so it
//! drops unknown keys silently, while the service behind `capital-changes` and
//! `listing-applicants` (reached through `MARKETDATA_SERVICE_API_URL`) runs with
//! `forbidNonWhitelisted` and answers 400. Same SDK, two behaviours — the table
//! gives callers one.
//!
//! The source of truth is the server's DTOs (fugle-realtime `apps/api-gateway`
//! and `apps/service-stock`), not developer.fugle.tw and not the builders here;
//! the tests in this module keep the builders and the table in step.
//!
//! **Keys only, never values.** The server's value rules are inconsistent
//! (`type=oddlot` is case-sensitive, the single-contract `session=afterhours`
//! is lower-cased first, the list and historical `session` are upper-cased to
//! `REGULAR|AFTERHOURS`), so copying them here would only drift. A bad value
//! gets the server's own error.
//!
//! Hidden from the docs and the public-API baseline like
//! [`RestClient::get_json`](crate::rest::RestClient::get_json): it serves the
//! bindings and changes whenever Fugle adds a parameter.

/// One query parameter of an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamSpec {
    /// The key on the wire, as the API documents it (`isTrial`, `start_date`).
    pub name: &'static str,
    /// The snake_case name the bindings use (`is_trial`); the typed builder
    /// method has this name unless [`methods`](Self::methods) says otherwise.
    pub canonical: &'static str,
    /// Builder methods that set this parameter when they are not named
    /// `canonical` (`type` is a Rust keyword, so `typ` / `type_filter`; the
    /// trades sort is `sort_asc` / `sort_desc`). Empty means `[canonical]`.
    pub methods: &'static [&'static str],
    /// Other spellings already in circulation that bindings accept
    /// (`oddLot`, this SDK's own camelCase for the odd-lot flag).
    pub aliases: &'static [&'static str],
    /// A boolean canonical that, when true, sends this literal under `name`:
    /// `odd_lot = true` is `type=oddlot`, `after_hours = true` is
    /// `session=afterhours`. `name` itself still takes the verbatim value.
    pub flag: Option<&'static str>,
    /// Recorded, not enforced: the builders stay optional so a client can be
    /// configured in steps; the server answers 400 when it is missing.
    pub required: bool,
    /// `name` is a Python keyword, so the Python binding also accepts it with
    /// a trailing underscore (`from_`). The rule lives there; only the fact
    /// lives here.
    pub python_keyword: bool,
}

const fn param(name: &'static str, canonical: &'static str) -> ParamSpec {
    ParamSpec {
        name,
        canonical,
        methods: &[],
        aliases: &[],
        flag: None,
        required: false,
        python_keyword: false,
    }
}

const fn required(name: &'static str, canonical: &'static str) -> ParamSpec {
    ParamSpec {
        required: true,
        ..param(name, canonical)
    }
}

/// One endpoint's full query contract.
#[derive(Debug, Clone, Copy)]
pub struct EndpointSpec {
    /// URL path segments before the path parameter, as the bindings already
    /// key their calls: `["stock", "intraday", "trades"]`.
    pub path: &'static [&'static str],
    /// The key a bindings caller uses for the last path segment (`symbol`,
    /// `market`), if the endpoint has one.
    pub path_param: Option<&'static str>,
    /// Other accepted keys for the path segment: futopt historical calls it
    /// `product` because a contract code there is a 404.
    pub path_param_aliases: &'static [&'static str],
    pub params: &'static [ParamSpec],
    /// The builder's source file under `core/src/rest/`, for the tests that
    /// compare the two.
    pub builder_src: &'static str,
}

/// How a caller's key matched an endpoint's parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolved {
    pub spec: &'static ParamSpec,
    /// The key was the flag form (`odd_lot`, `oddLot`, `after_hours`): the
    /// value is a boolean, and `true` sends `spec.flag` under `spec.name`.
    pub as_flag: bool,
}

impl EndpointSpec {
    /// The spec for a path such as `["stock", "intraday", "trades"]`.
    pub fn for_path(path: &[&str]) -> Option<&'static EndpointSpec> {
        ENDPOINTS.iter().find(|e| e.path == path)
    }

    /// The spec for a dotted name such as `stock.corporate_actions.capital_changes`
    /// (the Python binding's key); `_` and `-` are interchangeable.
    pub fn for_name(name: &str) -> Option<&'static EndpointSpec> {
        let segments: Vec<&str> = name.split('.').collect();
        ENDPOINTS.iter().find(|e| {
            e.path.len() == segments.len()
                && e.path
                    .iter()
                    .zip(&segments)
                    .all(|(a, b)| same_segment(a, b))
        })
    }

    /// Whether `key` names the path parameter (`symbol`, or `product` where
    /// that alias applies).
    pub fn is_path_param(&self, key: &str) -> bool {
        self.path_param == Some(key) || self.path_param_aliases.contains(&key)
    }

    /// Match a caller's key against the wire name, the canonical name and the
    /// aliases. `None` is an unknown key.
    pub fn resolve(&self, key: &str) -> Option<Resolved> {
        self.params.iter().find_map(|spec| {
            if spec.name == key {
                Some(Resolved {
                    spec,
                    as_flag: false,
                })
            } else if spec.canonical == key || spec.aliases.contains(&key) {
                Some(Resolved {
                    spec,
                    as_flag: spec.flag.is_some(),
                })
            } else {
                None
            }
        })
    }

    /// The accepted spelling an unknown key was probably meant as: the one
    /// that differs only in case (`ODDLOT` → `oddLot`), else the one that
    /// differs only in case and underscores (`istrial` → `isTrial`).
    ///
    /// It is the matched spelling, not the wire name, so the suggestion can
    /// be used as written: `oddlot` points at the flag `oddLot` (a boolean),
    /// not at `type`, which takes the string `'oddlot'`.
    pub fn suggest(&self, key: &str) -> Option<&'static str> {
        let spellings = || {
            self.params.iter().flat_map(|spec| {
                [spec.name, spec.canonical]
                    .into_iter()
                    .chain(spec.aliases.iter().copied())
            })
        };
        let wanted = fold(key);
        spellings()
            .find(|candidate| candidate.eq_ignore_ascii_case(key))
            .or_else(|| spellings().find(|candidate| fold(candidate) == wanted))
    }

    /// The wire names, for "accepted: …" in an error message.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.params.iter().map(|spec| spec.name)
    }
}

fn same_segment(path: &str, name: &str) -> bool {
    path.len() == name.len()
        && path
            .bytes()
            .zip(name.bytes())
            .all(|(a, b)| a == b || (a == b'-' && b == b'_'))
}

fn fold(key: &str) -> String {
    key.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

// ---------------------------------------------------------------------------
// Shared parameters
// ---------------------------------------------------------------------------

/// `type=oddlot` on the single-symbol stock intraday endpoints.
const ODD_LOT: ParamSpec = ParamSpec {
    aliases: &["oddLot"],
    flag: Some("oddlot"),
    ..param("type", "odd_lot")
};
/// `session=afterhours` on the single-contract futopt endpoints (lower case
/// there; the server lower-cases before checking).
const AFTER_HOURS: ParamSpec = ParamSpec {
    flag: Some("afterhours"),
    ..param("session", "after_hours")
};
/// `session=AFTERHOURS` on the futopt list endpoints, which upper-case and
/// accept `REGULAR|AFTERHOURS`; lower case there silently returns nothing.
const AFTER_HOURS_LIST: ParamSpec = ParamSpec {
    flag: Some("AFTERHOURS"),
    ..param("session", "after_hours")
};
const FROM: ParamSpec = ParamSpec {
    python_keyword: true,
    ..param("from", "from")
};
const TO: ParamSpec = param("to", "to");
const TIMEFRAME: ParamSpec = param("timeframe", "timeframe");
const SORT: ParamSpec = param("sort", "sort");
const FIELDS: ParamSpec = param("fields", "fields");
const OFFSET: ParamSpec = param("offset", "offset");
const LIMIT: ParamSpec = param("limit", "limit");
const IS_TRIAL: ParamSpec = param("isTrial", "is_trial");
const START_DATE: ParamSpec = param("start_date", "start_date");
const END_DATE: ParamSpec = param("end_date", "end_date");
const EXCHANGE: ParamSpec = param("exchange", "exchange");
/// `type` on the list endpoints (`EQUITY`, `FUTURE`, …), set by `typ()`.
const TYPE_LIST: ParamSpec = ParamSpec {
    methods: &["typ"],
    ..required("type", "type")
};
/// `type` on the snapshot endpoints (`ALLBUT0999|COMMONSTOCK`), set by `type_filter()`.
const TYPE_FILTER: ParamSpec = ParamSpec {
    methods: &["type_filter"],
    ..param("type", "type")
};
const CONTRACT_TYPE: ParamSpec = param("contractType", "contract_type");
const TECHNICAL_RANGE: [ParamSpec; 3] = [FROM, TO, TIMEFRAME];
const OWNERSHIP: &[ParamSpec] = &[FROM, TO, SORT];

const fn stock(section: &'static str, endpoint: &'static str) -> [&'static str; 3] {
    ["stock", section, endpoint]
}

const fn futopt(section: &'static str, endpoint: &'static str) -> [&'static str; 3] {
    ["futopt", section, endpoint]
}

macro_rules! endpoint {
    ($path:expr, $param:expr, $aliases:expr, $src:literal, [$($p:expr),* $(,)?]) => {
        EndpointSpec {
            path: &$path,
            path_param: $param,
            path_param_aliases: $aliases,
            params: &[$($p),*],
            builder_src: $src,
        }
    };
}

/// Every endpoint the typed builders cover, in `core/src/rest` order.
pub static ENDPOINTS: &[EndpointSpec] = &[
    // stock / intraday — src/stock/intraday/dto/*.dto.ts
    endpoint!(
        stock("intraday", "tickers"),
        None,
        &[],
        "stock/intraday/tickers.rs",
        [
            TYPE_LIST,
            EXCHANGE,
            param("market", "market"),
            param("industry", "industry"),
            param("isNormal", "is_normal"),
            param("isAttention", "is_attention"),
            param("isDisposition", "is_disposition"),
            param("isHalted", "is_halted"),
            param("symbol", "symbol"),
        ]
    ),
    endpoint!(
        stock("intraday", "ticker"),
        Some("symbol"),
        &[],
        "stock/intraday/ticker.rs",
        [ODD_LOT]
    ),
    endpoint!(
        stock("intraday", "quote"),
        Some("symbol"),
        &[],
        "stock/intraday/quote.rs",
        [ODD_LOT]
    ),
    endpoint!(
        stock("intraday", "candles"),
        Some("symbol"),
        &[],
        "stock/intraday/candles.rs",
        [TIMEFRAME, ODD_LOT, SORT,]
    ),
    endpoint!(
        stock("intraday", "trades"),
        Some("symbol"),
        &[],
        "stock/intraday/trades.rs",
        [
            ODD_LOT,
            OFFSET,
            LIMIT,
            ParamSpec {
                methods: &["sort_asc", "sort_desc"],
                ..SORT
            },
            IS_TRIAL,
        ]
    ),
    endpoint!(
        stock("intraday", "volumes"),
        Some("symbol"),
        &[],
        "stock/intraday/volumes.rs",
        [ODD_LOT]
    ),
    // stock / historical — src/stock/historical/dto/get-candles.dto.ts; stats takes no query
    endpoint!(
        stock("historical", "candles"),
        Some("symbol"),
        &[],
        "stock/historical/candles.rs",
        [
            FROM,
            TO,
            TIMEFRAME,
            FIELDS,
            SORT,
            param("adjusted", "adjusted"),
        ]
    ),
    endpoint!(
        stock("historical", "stats"),
        Some("symbol"),
        &[],
        "stock/historical/stats.rs",
        []
    ),
    // stock / snapshot — src/stock/snapshot/dto/*.dto.ts
    endpoint!(
        stock("snapshot", "quotes"),
        Some("market"),
        &[],
        "stock/snapshot/quotes.rs",
        [TYPE_FILTER]
    ),
    endpoint!(
        stock("snapshot", "movers"),
        Some("market"),
        &[],
        "stock/snapshot/movers.rs",
        [
            required("direction", "direction"),
            required("change", "change"),
            TYPE_FILTER,
            param("gt", "gt"),
            param("gte", "gte"),
            param("lt", "lt"),
            param("lte", "lte"),
            param("eq", "eq"),
        ]
    ),
    endpoint!(
        stock("snapshot", "actives"),
        Some("market"),
        &[],
        "stock/snapshot/actives.rs",
        [required("trade", "trade"), TYPE_FILTER,]
    ),
    // stock / technical — src/stock/technical/dto/*.dto.ts
    endpoint!(
        stock("technical", "sma"),
        Some("symbol"),
        &[],
        "stock/technical/sma.rs",
        [
            TECHNICAL_RANGE[0],
            TECHNICAL_RANGE[1],
            TECHNICAL_RANGE[2],
            required("period", "period"),
        ]
    ),
    endpoint!(
        stock("technical", "rsi"),
        Some("symbol"),
        &[],
        "stock/technical/rsi.rs",
        [
            TECHNICAL_RANGE[0],
            TECHNICAL_RANGE[1],
            TECHNICAL_RANGE[2],
            required("period", "period"),
        ]
    ),
    endpoint!(
        stock("technical", "kdj"),
        Some("symbol"),
        &[],
        "stock/technical/kdj.rs",
        [
            TECHNICAL_RANGE[0],
            TECHNICAL_RANGE[1],
            TECHNICAL_RANGE[2],
            required("rPeriod", "r_period"),
            required("kPeriod", "k_period"),
            required("dPeriod", "d_period"),
        ]
    ),
    endpoint!(
        stock("technical", "macd"),
        Some("symbol"),
        &[],
        "stock/technical/macd.rs",
        [
            TECHNICAL_RANGE[0],
            TECHNICAL_RANGE[1],
            TECHNICAL_RANGE[2],
            required("fast", "fast"),
            required("slow", "slow"),
            required("signal", "signal"),
        ]
    ),
    endpoint!(
        stock("technical", "bb"),
        Some("symbol"),
        &[],
        "stock/technical/bb.rs",
        [
            TECHNICAL_RANGE[0],
            TECHNICAL_RANGE[1],
            TECHNICAL_RANGE[2],
            required("period", "period"),
        ]
    ),
    // stock / corporate-actions — dividends: service-stock dividend-query.dto.ts;
    // the other two are forwarded to a service outside the repo that answers
    // 400 to any key not below (verified against prod in #168).
    endpoint!(
        stock("corporate-actions", "dividends"),
        None,
        &[],
        "stock/corporate_actions/dividends.rs",
        [START_DATE, END_DATE, EXCHANGE, SORT,]
    ),
    endpoint!(
        stock("corporate-actions", "listing-applicants"),
        None,
        &[],
        "stock/corporate_actions/listing_applicants.rs",
        [START_DATE, END_DATE, EXCHANGE, SORT,]
    ),
    endpoint!(
        stock("corporate-actions", "capital-changes"),
        None,
        &[],
        "stock/corporate_actions/capital_changes.rs",
        [START_DATE, END_DATE, SORT,]
    ),
    // stock / ownership — service-stock ownership/dto/*.dto.ts, one contract for all four
    EndpointSpec {
        path: &stock("ownership", "etf-holdings"),
        path_param: Some("symbol"),
        path_param_aliases: &[],
        params: OWNERSHIP,
        builder_src: "stock/ownership/etf_holdings.rs",
    },
    EndpointSpec {
        path: &stock("ownership", "institutional-trades"),
        path_param: Some("symbol"),
        path_param_aliases: &[],
        params: OWNERSHIP,
        builder_src: "stock/ownership/institutional_trades.rs",
    },
    EndpointSpec {
        path: &stock("ownership", "director-holdings"),
        path_param: Some("symbol"),
        path_param_aliases: &[],
        params: OWNERSHIP,
        builder_src: "stock/ownership/director_holdings.rs",
    },
    EndpointSpec {
        path: &stock("ownership", "tdcc-distribution"),
        path_param: Some("symbol"),
        path_param_aliases: &[],
        params: OWNERSHIP,
        builder_src: "stock/ownership/tdcc_distribution.rs",
    },
    // futopt / intraday — src/futopt/intraday/dto/*.dto.ts. `products` takes
    // `status` and not `product`; `tickers` the other way round.
    endpoint!(
        futopt("intraday", "products"),
        None,
        &[],
        "futopt/intraday/products.rs",
        [
            TYPE_LIST,
            EXCHANGE,
            AFTER_HOURS_LIST,
            CONTRACT_TYPE,
            param("status", "status"),
        ]
    ),
    endpoint!(
        futopt("intraday", "tickers"),
        None,
        &[],
        "futopt/intraday/tickers.rs",
        [
            TYPE_LIST,
            EXCHANGE,
            AFTER_HOURS_LIST,
            param("product", "product"),
            CONTRACT_TYPE,
            param("isSpread", "is_spread"),
        ]
    ),
    endpoint!(
        futopt("intraday", "ticker"),
        Some("symbol"),
        &[],
        "futopt/intraday/ticker.rs",
        [AFTER_HOURS]
    ),
    endpoint!(
        futopt("intraday", "quote"),
        Some("symbol"),
        &[],
        "futopt/intraday/quote.rs",
        [AFTER_HOURS]
    ),
    endpoint!(
        futopt("intraday", "candles"),
        Some("symbol"),
        &[],
        "futopt/intraday/candles.rs",
        [AFTER_HOURS, TIMEFRAME,]
    ),
    endpoint!(
        futopt("intraday", "trades"),
        Some("symbol"),
        &[],
        "futopt/intraday/trades.rs",
        [AFTER_HOURS, OFFSET, LIMIT, IS_TRIAL,]
    ),
    endpoint!(
        futopt("intraday", "volumes"),
        Some("symbol"),
        &[],
        "futopt/intraday/volumes.rs",
        [AFTER_HOURS]
    ),
    // futopt / historical — src/futopt/historical/dto/*.dto.ts; the path
    // segment is a product, and the bindings accept `product` for it.
    endpoint!(
        futopt("historical", "candles"),
        Some("symbol"),
        &["product"],
        "futopt/historical/candles.rs",
        [
            FROM,
            TO,
            param("contractMonth", "contract_month"),
            FIELDS,
            TIMEFRAME,
            SORT,
            param("strikePrice", "strike_price"),
            param("callPut", "call_put"),
            AFTER_HOURS,
        ]
    ),
    endpoint!(
        futopt("historical", "daily"),
        Some("symbol"),
        &["product"],
        "futopt/historical/daily.rs",
        [param("date", "date"), AFTER_HOURS,]
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::futopt::{ContractType, FutOptType};
    use crate::rest::stock::ownership::HoldingsSort;
    use crate::rest::{Auth, RestClient};
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    // -- lookup -------------------------------------------------------------

    #[test]
    fn covers_the_thirty_two_typed_endpoints_once_each() {
        assert_eq!(ENDPOINTS.len(), 32);
        let paths: BTreeSet<_> = ENDPOINTS.iter().map(|e| e.path).collect();
        assert_eq!(paths.len(), ENDPOINTS.len(), "duplicate path");
        let sources: BTreeSet<_> = ENDPOINTS.iter().map(|e| e.builder_src).collect();
        assert_eq!(sources.len(), ENDPOINTS.len(), "duplicate builder_src");
    }

    #[test]
    fn for_path_and_for_name_agree() {
        let trades = EndpointSpec::for_path(&["stock", "intraday", "trades"]).unwrap();
        assert_eq!(trades.path_param, Some("symbol"));
        assert!(std::ptr::eq(
            trades,
            EndpointSpec::for_name("stock.intraday.trades").unwrap()
        ));

        let capital = EndpointSpec::for_name("stock.corporate_actions.capital_changes").unwrap();
        assert_eq!(
            capital.path,
            ["stock", "corporate-actions", "capital-changes"]
        );
        assert!(EndpointSpec::for_name("stock.corporate-actions.capital-changes").is_some());

        assert!(EndpointSpec::for_path(&["stock", "intraday"]).is_none());
        assert!(EndpointSpec::for_name("stock.intraday.quotes").is_none());
    }

    #[test]
    fn every_parameter_has_a_unique_key_set_within_its_endpoint() {
        for e in ENDPOINTS {
            for spec in e.params {
                assert!(
                    !e.is_path_param(spec.name),
                    "{}: `{}` is also the path param",
                    e.path.join("/"),
                    spec.name
                );
            }
            // The wire name and the canonical may coincide (`from`), but two
            // different parameters must never share a key.
            let keys: Vec<&str> = e
                .params
                .iter()
                .flat_map(|s| {
                    [s.name, s.canonical]
                        .into_iter()
                        .chain(s.aliases.iter().copied())
                })
                .collect();
            for key in &keys {
                let owners: BTreeSet<&str> = e
                    .params
                    .iter()
                    .filter(|s| s.name == *key || s.canonical == *key || s.aliases.contains(key))
                    .map(|s| s.name)
                    .collect();
                assert_eq!(
                    owners.len(),
                    1,
                    "{}: `{key}` resolves to {owners:?}",
                    e.path.join("/")
                );
            }
        }
    }

    #[test]
    fn resolve_distinguishes_wire_flag_and_alias_forms() {
        let quote = EndpointSpec::for_path(&["stock", "intraday", "quote"]).unwrap();
        let wire = quote.resolve("type").unwrap();
        assert_eq!((wire.spec.name, wire.as_flag), ("type", false));
        let flag = quote.resolve("odd_lot").unwrap();
        assert_eq!((flag.spec.flag, flag.as_flag), (Some("oddlot"), true));
        let alias = quote.resolve("oddLot").unwrap();
        assert_eq!((alias.spec.name, alias.as_flag), ("type", true));
        assert!(
            quote.resolve("symbol").is_none(),
            "the path param is not a query key"
        );
        assert!(quote.is_path_param("symbol"));

        let trades = EndpointSpec::for_path(&["stock", "intraday", "trades"]).unwrap();
        let trial = trades.resolve("is_trial").unwrap();
        assert_eq!((trial.spec.name, trial.as_flag), ("isTrial", false));
        assert!(trades.resolve("isTrail").is_none());

        let list = EndpointSpec::for_path(&["futopt", "intraday", "tickers"]).unwrap();
        assert_eq!(
            list.resolve("after_hours").unwrap().spec.flag,
            Some("AFTERHOURS")
        );
        assert!(
            list.resolve("status").is_none(),
            "status belongs to products, not tickers"
        );
        let products = EndpointSpec::for_path(&["futopt", "intraday", "products"]).unwrap();
        assert!(
            products.resolve("product").is_none(),
            "product belongs to tickers, not products"
        );

        let hist = EndpointSpec::for_path(&["futopt", "historical", "candles"]).unwrap();
        assert!(hist.is_path_param("product") && hist.is_path_param("symbol"));
        assert!(hist.resolve("from").unwrap().spec.python_keyword);
    }

    #[test]
    fn suggest_points_a_near_miss_at_a_usable_spelling() {
        let trades = EndpointSpec::for_path(&["stock", "intraday", "trades"]).unwrap();
        assert_eq!(trades.suggest("istrial"), Some("isTrial"));
        assert_eq!(trades.suggest("Is_Trial"), Some("is_trial"), "case alone: the snake_case spelling");
        assert_eq!(trades.suggest("IsTrial"), Some("isTrial"));
        assert_eq!(trades.suggest("limits"), None);
        assert_eq!(trades.suggest("sortAsc"), None, "the trades sort has no such spelling");

        // A flag's spelling, never the wire name it sets: `type: true` would
        // be wrong, `oddLot: true` and `odd_lot: true` are right.
        assert_eq!(trades.suggest("oddlot"), Some("oddLot"));
        assert_eq!(trades.suggest("ODDLOT"), Some("oddLot"));
        assert_eq!(trades.suggest("odd_Lot"), Some("odd_lot"));
        let quote = EndpointSpec::for_path(&["futopt", "intraday", "quote"]).unwrap();
        assert_eq!(quote.suggest("afterhours"), Some("after_hours"));
        assert_eq!(quote.suggest("AfterHours"), Some("after_hours"));
        assert_eq!(
            trades.names().collect::<Vec<_>>(),
            ["type", "offset", "limit", "sort", "isTrial"]
        );
    }

    // -- builders vs table --------------------------------------------------

    fn builder_source(e: &EndpointSpec) -> String {
        let path = format!("{}/src/rest/{}", env!("CARGO_MANIFEST_DIR"), e.builder_src);
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
    }

    /// `pub fn name(mut self, …)` — the setters; `send`, `new` and the like
    /// take `self` by value or are not `pub`.
    fn setters(source: &str) -> BTreeSet<String> {
        source
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("pub fn "))
            .filter_map(|rest| {
                let (name, args) = rest.split_once('(')?;
                args.starts_with("mut self").then(|| name.to_string())
            })
            .collect()
    }

    #[test]
    fn every_table_entry_is_a_builder_setter_and_vice_versa() {
        for e in ENDPOINTS {
            let source = builder_source(e);
            let mut setters = setters(&source);
            if let Some(path_param) = e.path_param {
                assert!(
                    setters.remove(path_param),
                    "{}: no `{path_param}()` setter",
                    e.builder_src
                );
            }
            let mut expected = BTreeSet::new();
            for spec in e.params {
                let methods: Vec<&str> = if spec.methods.is_empty() {
                    vec![spec.canonical]
                } else {
                    spec.methods.to_vec()
                };
                for method in methods {
                    assert!(
                        setters.contains(method),
                        "{}: table lists `{}` but the builder has no `{method}()`",
                        e.builder_src,
                        spec.name
                    );
                    expected.insert(method.to_string());
                }
            }
            let extra: Vec<_> = setters.difference(&expected).collect();
            assert!(
                extra.is_empty(),
                "{}: builder setters missing from the table: {extra:?}",
                e.builder_src
            );
        }
    }

    // -- what send() really puts on the wire --------------------------------

    /// Answers `{}` to everything and keeps each request line.
    fn loopback() -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let seen = lines.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                while let Ok(n) = stream.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..end]).into_owned();
                        seen.lock()
                            .unwrap()
                            .push(head.lines().next().unwrap().to_string());
                        let _ = stream.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                        );
                        break;
                    }
                }
            }
        });
        (format!("http://127.0.0.1:{port}/marketdata"), lines)
    }

    /// Path segments (after the version) and query keys of `GET /marketdata/v1.0/... HTTP/1.1`.
    fn parse(request_line: &str) -> (Vec<String>, BTreeMap<String, String>) {
        let target = request_line.split(' ').nth(1).unwrap();
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let segments = path
            .trim_start_matches("/marketdata/v1.0/")
            .split('/')
            .map(str::to_string)
            .collect();
        let query = query
            .split('&')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let (k, v) = p.split_once('=').unwrap_or((p, ""));
                (k.to_string(), v.to_string())
            })
            .collect();
        (segments, query)
    }

    type Send = fn(&RestClient) -> Result<serde_json::Value, crate::errors::MarketDataError>;

    /// Each endpoint with every parameter set, keyed by path, so the
    /// request the builder makes can be compared with the table.
    fn full_calls() -> Vec<(&'static [&'static str], Send)> {
        vec![
            (&["stock", "intraday", "tickers"], |c| {
                c.stock()
                    .intraday()
                    .tickers()
                    .typ("EQUITY")
                    .exchange("TWSE")
                    .market("TSE")
                    .industry("24")
                    .is_normal(true)
                    .is_attention(true)
                    .is_disposition(false)
                    .is_halted(false)
                    .symbol("2330,2317")
                    .send()
            }),
            (&["stock", "intraday", "ticker"], |c| {
                c.stock()
                    .intraday()
                    .ticker()
                    .symbol("2330")
                    .odd_lot(true)
                    .send()
            }),
            (&["stock", "intraday", "quote"], |c| {
                c.stock()
                    .intraday()
                    .quote()
                    .symbol("2330")
                    .odd_lot(true)
                    .send()
            }),
            (&["stock", "intraday", "candles"], |c| {
                c.stock()
                    .intraday()
                    .candles()
                    .symbol("2330")
                    .timeframe("5")
                    .odd_lot(true)
                    .sort("asc")
                    .send()
            }),
            (&["stock", "intraday", "trades"], |c| {
                c.stock()
                    .intraday()
                    .trades()
                    .symbol("2330")
                    .odd_lot(true)
                    .offset(10)
                    .limit(5)
                    .sort_asc()
                    .is_trial(true)
                    .send()
            }),
            (&["stock", "intraday", "volumes"], |c| {
                c.stock()
                    .intraday()
                    .volumes()
                    .symbol("2330")
                    .odd_lot(true)
                    .send()
            }),
            (&["stock", "historical", "candles"], |c| {
                c.stock()
                    .historical()
                    .candles()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .timeframe("D")
                    .fields("open,close")
                    .sort("asc")
                    .adjusted(true)
                    .send()
            }),
            (&["stock", "historical", "stats"], |c| {
                c.stock().historical().stats().symbol("2330").send()
            }),
            (&["stock", "snapshot", "quotes"], |c| {
                c.stock()
                    .snapshot()
                    .quotes()
                    .market("TSE")
                    .type_filter("COMMONSTOCK")
                    .send()
            }),
            (&["stock", "snapshot", "movers"], |c| {
                c.stock()
                    .snapshot()
                    .movers()
                    .market("TSE")
                    .direction("up")
                    .change("percent")
                    .type_filter("COMMONSTOCK")
                    .gt(1.0)
                    .gte(2.0)
                    .lt(9.0)
                    .lte(10.0)
                    .eq(5.0)
                    .send()
            }),
            (&["stock", "snapshot", "actives"], |c| {
                c.stock()
                    .snapshot()
                    .actives()
                    .market("TSE")
                    .trade("volume")
                    .type_filter("COMMONSTOCK")
                    .send()
            }),
            (&["stock", "technical", "sma"], |c| {
                c.stock()
                    .technical()
                    .sma()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .timeframe("D")
                    .period(20)
                    .send()
            }),
            (&["stock", "technical", "rsi"], |c| {
                c.stock()
                    .technical()
                    .rsi()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .timeframe("D")
                    .period(14)
                    .send()
            }),
            (&["stock", "technical", "kdj"], |c| {
                c.stock()
                    .technical()
                    .kdj()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .timeframe("D")
                    .r_period(9)
                    .k_period(3)
                    .d_period(3)
                    .send()
            }),
            (&["stock", "technical", "macd"], |c| {
                c.stock()
                    .technical()
                    .macd()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .timeframe("D")
                    .fast(12)
                    .slow(26)
                    .signal(9)
                    .send()
            }),
            (&["stock", "technical", "bb"], |c| {
                c.stock()
                    .technical()
                    .bb()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .timeframe("D")
                    .period(20)
                    .send()
            }),
            (&["stock", "corporate-actions", "dividends"], |c| {
                c.stock()
                    .corporate_actions()
                    .dividends()
                    .start_date("2026-01-01")
                    .end_date("2026-02-01")
                    .exchange("TWSE")
                    .sort("asc")
                    .send()
            }),
            (&["stock", "corporate-actions", "listing-applicants"], |c| {
                c.stock()
                    .corporate_actions()
                    .listing_applicants()
                    .start_date("2026-01-01")
                    .end_date("2026-02-01")
                    .exchange("TWSE")
                    .sort("asc")
                    .send()
            }),
            (&["stock", "corporate-actions", "capital-changes"], |c| {
                c.stock()
                    .corporate_actions()
                    .capital_changes()
                    .start_date("2026-01-01")
                    .end_date("2026-02-01")
                    .sort("asc")
                    .send()
            }),
            (&["stock", "ownership", "etf-holdings"], |c| {
                c.stock()
                    .ownership()
                    .etf_holdings()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .sort(HoldingsSort::Asc)
                    .send()
            }),
            (&["stock", "ownership", "institutional-trades"], |c| {
                c.stock()
                    .ownership()
                    .institutional_trades()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .sort(HoldingsSort::Asc)
                    .send()
            }),
            (&["stock", "ownership", "director-holdings"], |c| {
                c.stock()
                    .ownership()
                    .director_holdings()
                    .symbol("2330")
                    .from("2026-01")
                    .to("2026-02")
                    .sort(HoldingsSort::Asc)
                    .send()
            }),
            (&["stock", "ownership", "tdcc-distribution"], |c| {
                c.stock()
                    .ownership()
                    .tdcc_distribution()
                    .symbol("2330")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .sort(HoldingsSort::Asc)
                    .send()
            }),
            (&["futopt", "intraday", "products"], |c| {
                c.futopt()
                    .intraday()
                    .products()
                    .typ(FutOptType::Future)
                    .exchange("TAIFEX")
                    .after_hours()
                    .contract_type(ContractType::Index)
                    .status("N")
                    .send()
            }),
            (&["futopt", "intraday", "tickers"], |c| {
                c.futopt()
                    .intraday()
                    .tickers()
                    .typ(FutOptType::Future)
                    .exchange("TAIFEX")
                    .after_hours()
                    .product("TXF")
                    .contract_type(ContractType::Index)
                    .is_spread(false)
                    .send()
            }),
            (&["futopt", "intraday", "ticker"], |c| {
                c.futopt()
                    .intraday()
                    .ticker()
                    .symbol("TXFD6")
                    .after_hours()
                    .send()
            }),
            (&["futopt", "intraday", "quote"], |c| {
                c.futopt()
                    .intraday()
                    .quote()
                    .symbol("TXFD6")
                    .after_hours()
                    .send()
            }),
            (&["futopt", "intraday", "candles"], |c| {
                c.futopt()
                    .intraday()
                    .candles()
                    .symbol("TXFD6")
                    .after_hours()
                    .timeframe("5")
                    .send()
            }),
            (&["futopt", "intraday", "trades"], |c| {
                c.futopt()
                    .intraday()
                    .trades()
                    .symbol("TXFD6")
                    .after_hours()
                    .offset(10)
                    .limit(5)
                    .is_trial(true)
                    .send()
            }),
            (&["futopt", "intraday", "volumes"], |c| {
                c.futopt()
                    .intraday()
                    .volumes()
                    .symbol("TXFD6")
                    .after_hours()
                    .send()
            }),
            (&["futopt", "historical", "candles"], |c| {
                c.futopt()
                    .historical()
                    .candles()
                    .symbol("TXO")
                    .from("2026-01-01")
                    .to("2026-02-01")
                    .contract_month("1!")
                    .fields("open,close")
                    .timeframe("D")
                    .sort("asc")
                    .strike_price(23000.0)
                    .call_put("CALL")
                    .after_hours(true)
                    .send()
            }),
            (&["futopt", "historical", "daily"], |c| {
                c.futopt()
                    .historical()
                    .daily()
                    .symbol("TXF")
                    .date("2026-01-15")
                    .after_hours(true)
                    .send()
            }),
        ]
    }

    #[test]
    fn every_key_a_builder_sends_is_in_its_table_entry_and_the_table_has_no_dead_keys() {
        let (base, lines) = loopback();
        let client = RestClient::new(Auth::ApiKey("test".to_string())).base_url(&base);
        let calls = full_calls();
        assert_eq!(calls.len(), ENDPOINTS.len(), "a full call per endpoint");

        for (path, send) in calls {
            let spec = EndpointSpec::for_path(path)
                .unwrap_or_else(|| panic!("{path:?} is not in the table"));
            send(&client).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            let line = lines.lock().unwrap().pop().unwrap();
            let (segments, query) = parse(&line);

            let expected_len = path.len() + usize::from(spec.path_param.is_some());
            assert_eq!(segments.len(), expected_len, "{path:?}: {line}");
            assert_eq!(&segments[..path.len()], path, "{line}");

            let sent: BTreeSet<&str> = query.keys().map(String::as_str).collect();
            let listed: BTreeSet<&str> = spec.names().collect();
            let unlisted: Vec<_> = sent.difference(&listed).collect();
            assert!(
                unlisted.is_empty(),
                "{path:?} sends {unlisted:?} which the table does not list: {line}"
            );
            let unsent: Vec<_> = listed.difference(&sent).collect();
            assert!(
                unsent.is_empty(),
                "{path:?}: the table lists {unsent:?} but the full call did not send it: {line}"
            );

            for spec in spec.params {
                if let Some(flag) = spec.flag {
                    assert_eq!(
                        query[spec.name], flag,
                        "{path:?}: `{}` flag value",
                        spec.name
                    );
                }
            }
        }
    }
}
