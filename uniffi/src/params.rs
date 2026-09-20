//! Query-parameter records for the REST methods (#202).
//!
//! One `uniffi::Record` per parameter set in `core::rest::params::ENDPOINTS`;
//! endpoints with the same set share a record (`OddLotParams` serves ticker,
//! quote and volumes). Every field is optional and unset means "not sent",
//! so a zero-initialised record (C++ `StockTradesParams{}`, Go `nil`, C#
//! `new StockTradesParams()`, Java `null`) sends nothing. Required
//! parameters (`type` on the list endpoints, `direction` / `change` on
//! movers, the technical periods) are positional arguments of the client
//! method instead, so they cannot be left out.
//!
//! The records list fields by canonical name; [`to_query`] resolves each
//! through the endpoint's table entry, which supplies the wire key
//! (`is_trial` → `isTrial`) and the flag literal (`odd_lot = true` →
//! `type=oddlot`; `after_hours = true` → `session=afterhours` on the single-
//! contract endpoints and `session=AFTERHOURS` on the list ones). Nothing on
//! the wire is spelled here, so the table stays the only place a key lives.
//! A field the endpoint's entry does not have — `exchange` on
//! capital-changes — is 1005 `INVALID_PARAMETER`, as in the js and py
//! bindings.
//!
//! Keys only, never values: a value is sent as given and the server checks
//! it. An `Option<String>` is sent even when empty.

use marketdata_core::rest::params::EndpointSpec;

use crate::errors::MarketDataError;

/// A record field's value, before the table decides how it is sent.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    /// Sent verbatim.
    Str(String),
    /// Sent as `true` / `false`.
    Bool(bool),
    /// A table flag: `true` sends the table's literal, `false` sends nothing.
    Flag(bool),
    /// Sent in decimal.
    U32(u32),
    /// Sent with Rust's `{}` (`2380.5` → `2380.5`, `2380.0` → `2380`); the
    /// locale never applies.
    F64(f64),
}

/// A field to send: the canonical name and the value.
pub(crate) type Field = (&'static str, Value);

/// One query pair as `RestClient::get_json` takes it.
pub(crate) type Pair = (&'static str, String);

/// A params record: the set fields, by canonical name.
pub(crate) trait QueryParams: Default {
    fn fields(&self) -> Vec<Field>;
}

/// The fields of an optional record; `None` is the default record, which
/// sends nothing.
pub(crate) fn fields_of<P: QueryParams>(params: Option<P>) -> Vec<Field> {
    params.unwrap_or_default().fields()
}

/// Resolve `fields` through `spec` into query pairs, in the order given.
///
/// A canonical name `spec` does not list is 1005 `INVALID_PARAMETER`.
pub(crate) fn to_query(spec: &EndpointSpec, fields: Vec<Field>) -> Result<Vec<Pair>, MarketDataError> {
    let mut pairs = Vec::with_capacity(fields.len());
    for (canonical, value) in fields {
        let Some(resolved) = spec.resolve(canonical) else {
            let accepted: Vec<&str> = spec.names().collect();
            return Err(marketdata_core::MarketDataError::InvalidParameter {
                name: canonical.to_string(),
                reason: format!(
                    "`{}` does not accept `{canonical}`; accepted keys: {}",
                    spec.path.join("."),
                    accepted.join(", ")
                ),
            }
            .into());
        };
        let name = resolved.spec.name;
        match value {
            Value::Str(s) => pairs.push((name, s)),
            Value::Bool(b) => pairs.push((name, b.to_string())),
            Value::Flag(true) => {
                let flag = resolved
                    .spec
                    .flag
                    .unwrap_or_else(|| panic!("`{canonical}` is not a flag in {}", spec.path.join("/")));
                pairs.push((name, flag.to_string()));
            }
            Value::Flag(false) => {}
            Value::U32(n) => pairs.push((name, n.to_string())),
            Value::F64(f) => pairs.push((name, f.to_string())),
        }
    }
    Ok(pairs)
}

/// Push the set fields of a record in declaration order.
macro_rules! fields {
    ($self:ident; $($name:ident : $kind:ident),* $(,)?) => {{
        let mut out: Vec<Field> = Vec::new();
        $(
            if let Some(v) = $self.$name.clone() {
                out.push((stringify!($name), Value::$kind(v)));
            }
        )*
        out
    }};
}

// ---------------------------------------------------------------------------
// Stock
// ---------------------------------------------------------------------------

/// Filters for `stock/intraday/tickers`; `type` is the method's argument.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct StockTickersParams {
    /// `TWSE` or `TPEx`.
    #[uniffi(default = None)]
    pub exchange: Option<String>,
    /// `TSE`, `OTC`, `ESB`, `TIB` or `PSB`.
    #[uniffi(default = None)]
    pub market: Option<String>,
    /// Industry code.
    #[uniffi(default = None)]
    pub industry: Option<String>,
    #[uniffi(default = None)]
    pub is_normal: Option<bool>,
    #[uniffi(default = None)]
    pub is_attention: Option<bool>,
    #[uniffi(default = None)]
    pub is_disposition: Option<bool>,
    #[uniffi(default = None)]
    pub is_halted: Option<bool>,
    /// Symbol prefix.
    #[uniffi(default = None)]
    pub symbol: Option<String>,
}

impl QueryParams for StockTickersParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self;
            exchange: Str, market: Str, industry: Str,
            is_normal: Bool, is_attention: Bool, is_disposition: Bool, is_halted: Bool,
            symbol: Str,
        )
    }
}

/// The odd-lot session flag for `stock/intraday/ticker`, `quote` and `volumes`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct OddLotParams {
    /// `true` asks for the intraday odd-lot session (`type=oddlot`).
    #[uniffi(default = None)]
    pub odd_lot: Option<bool>,
}

impl QueryParams for OddLotParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; odd_lot: Flag)
    }
}

/// Parameters for `stock/intraday/candles`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct StockCandlesParams {
    /// `1`, `5`, `10`, `15`, `30` or `60` minutes; unset takes the server
    /// default.
    #[uniffi(default = None)]
    pub timeframe: Option<String>,
    /// `true` asks for the intraday odd-lot session (`type=oddlot`).
    #[uniffi(default = None)]
    pub odd_lot: Option<bool>,
    /// `asc` or `desc`.
    #[uniffi(default = None)]
    pub sort: Option<String>,
}

impl QueryParams for StockCandlesParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; timeframe: Str, odd_lot: Flag, sort: Str)
    }
}

/// Parameters for `stock/intraday/trades`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct StockTradesParams {
    /// `true` asks for the intraday odd-lot session (`type=oddlot`).
    #[uniffi(default = None)]
    pub odd_lot: Option<bool>,
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    #[uniffi(default = None)]
    pub limit: Option<u32>,
    /// `asc` or `desc`.
    #[uniffi(default = None)]
    pub sort: Option<String>,
    #[uniffi(default = None)]
    pub is_trial: Option<bool>,
}

impl QueryParams for StockTradesParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; odd_lot: Flag, offset: U32, limit: U32, sort: Str, is_trial: Bool)
    }
}

/// Parameters for `stock/historical/candles`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct StockHistoricalCandlesParams {
    /// Start date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub from: Option<String>,
    /// End date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub to: Option<String>,
    /// `D`, `W`, `M`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
    #[uniffi(default = None)]
    pub timeframe: Option<String>,
    /// Comma-separated field names, `open,high,low,close,volume`.
    #[uniffi(default = None)]
    pub fields: Option<String>,
    /// `asc` or `desc`.
    #[uniffi(default = None)]
    pub sort: Option<String>,
    /// Adjusted prices.
    #[uniffi(default = None)]
    pub adjusted: Option<bool>,
}

impl QueryParams for StockHistoricalCandlesParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; from: Str, to: Str, timeframe: Str, fields: Str, sort: Str, adjusted: Bool)
    }
}

/// Parameters for `stock/snapshot/quotes` and `actives`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct SnapshotParams {
    /// `type`: `ALL`, `ALLBUT0999` or `COMMONSTOCK`.
    #[uniffi(default = None)]
    pub type_filter: Option<String>,
}

impl QueryParams for SnapshotParams {
    fn fields(&self) -> Vec<Field> {
        self.type_filter.clone().map(|v| ("type", Value::Str(v))).into_iter().collect()
    }
}

/// Parameters for `stock/snapshot/movers`; `direction` and `change` are the
/// method's arguments.
///
/// The price bounds are `f64`, and 0 is a bound like any other: unlike the
/// config records, an unset field is `None`, not 0.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct MoversParams {
    /// `type`: `ALL`, `ALLBUT0999` or `COMMONSTOCK`.
    #[uniffi(default = None)]
    pub type_filter: Option<String>,
    /// Change greater than.
    #[uniffi(default = None)]
    pub gt: Option<f64>,
    /// Change greater than or equal to.
    #[uniffi(default = None)]
    pub gte: Option<f64>,
    /// Change less than.
    #[uniffi(default = None)]
    pub lt: Option<f64>,
    /// Change less than or equal to.
    #[uniffi(default = None)]
    pub lte: Option<f64>,
    /// Change equal to.
    #[uniffi(default = None)]
    pub eq: Option<f64>,
}

impl QueryParams for MoversParams {
    fn fields(&self) -> Vec<Field> {
        let mut out = SnapshotParams { type_filter: self.type_filter.clone() }.fields();
        out.extend(fields!(self; gt: F64, gte: F64, lt: F64, lte: F64, eq: F64));
        out
    }
}

/// The date range for the `stock/technical` endpoints; the periods are the
/// method's arguments.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct TechnicalParams {
    /// Start date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub from: Option<String>,
    /// End date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub to: Option<String>,
    /// `D`, `W`, `M`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
    #[uniffi(default = None)]
    pub timeframe: Option<String>,
}

impl QueryParams for TechnicalParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; from: Str, to: Str, timeframe: Str)
    }
}

/// Parameters for the three `stock/corporate-actions` endpoints.
///
/// `capital-changes` has no `exchange`: setting it there is 1005
/// `INVALID_PARAMETER`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct CorporateActionsParams {
    /// `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub end_date: Option<String>,
    /// `TWSE` or `TPEx` (dividends and listing-applicants only).
    #[uniffi(default = None)]
    pub exchange: Option<String>,
    /// `asc` or `desc`.
    #[uniffi(default = None)]
    pub sort: Option<String>,
}

impl QueryParams for CorporateActionsParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; start_date: Str, end_date: Str, exchange: Str, sort: Str)
    }
}

/// Parameters for the four `stock/ownership` endpoints.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct OwnershipParams {
    /// Start date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub from: Option<String>,
    /// End date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub to: Option<String>,
    /// `asc` or `desc`.
    #[uniffi(default = None)]
    pub sort: Option<String>,
}

impl QueryParams for OwnershipParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; from: Str, to: Str, sort: Str)
    }
}

// ---------------------------------------------------------------------------
// FutOpt
// ---------------------------------------------------------------------------

/// Filters for `futopt/intraday/products`; `type` is the method's argument.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FutOptProductsParams {
    /// `TAIFEX`.
    #[uniffi(default = None)]
    pub exchange: Option<String>,
    /// `true` asks for the after-hours session (`session=AFTERHOURS`);
    /// unset or `false` is the regular session.
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
    /// `I`, `R`, `B`, `C`, `S` or `E`.
    #[uniffi(default = None)]
    pub contract_type: Option<String>,
    /// `N` (normal) or `U` (unlisted).
    #[uniffi(default = None)]
    pub status: Option<String>,
}

impl QueryParams for FutOptProductsParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; exchange: Str, after_hours: Flag, contract_type: Str, status: Str)
    }
}

/// Filters for `futopt/intraday/tickers`; `type` is the method's argument.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FutOptTickersParams {
    /// `TAIFEX`.
    #[uniffi(default = None)]
    pub exchange: Option<String>,
    /// `true` asks for the after-hours session (`session=AFTERHOURS`);
    /// unset or `false` is the regular session.
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
    /// Product code, `TXF`.
    #[uniffi(default = None)]
    pub product: Option<String>,
    /// `I`, `R`, `B`, `C`, `S` or `E`.
    #[uniffi(default = None)]
    pub contract_type: Option<String>,
    #[uniffi(default = None)]
    pub is_spread: Option<bool>,
}

impl QueryParams for FutOptTickersParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; exchange: Str, after_hours: Flag, product: Str, contract_type: Str, is_spread: Bool)
    }
}

/// The after-hours session flag for `futopt/intraday/ticker`, `quote` and
/// `volumes`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct AfterHoursParams {
    /// `true` asks for the after-hours session (`session=afterhours`);
    /// unset or `false` is the regular session.
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
}

impl QueryParams for AfterHoursParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; after_hours: Flag)
    }
}

/// Parameters for `futopt/intraday/candles`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FutOptCandlesParams {
    /// `true` asks for the after-hours session (`session=afterhours`).
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
    /// `1`, `5`, `10`, `15`, `30` or `60` minutes.
    #[uniffi(default = None)]
    pub timeframe: Option<String>,
}

impl QueryParams for FutOptCandlesParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; after_hours: Flag, timeframe: Str)
    }
}

/// Parameters for `futopt/intraday/trades`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FutOptTradesParams {
    /// `true` asks for the after-hours session (`session=afterhours`).
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    #[uniffi(default = None)]
    pub limit: Option<u32>,
    #[uniffi(default = None)]
    pub is_trial: Option<bool>,
}

impl QueryParams for FutOptTradesParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; after_hours: Flag, offset: U32, limit: U32, is_trial: Bool)
    }
}

/// Parameters for `futopt/historical/candles`.
///
/// `strike_price` is `f64`, and 0 is a strike like any other: unlike the
/// config records, an unset field is `None`, not 0.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FutOptHistoricalCandlesParams {
    /// Start date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub from: Option<String>,
    /// End date, `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub to: Option<String>,
    /// `YYYYMM`, or a continuous contract: `1!` (the server default), `2!`,
    /// `3!`.
    #[uniffi(default = None)]
    pub contract_month: Option<String>,
    /// Comma-separated field names.
    #[uniffi(default = None)]
    pub fields: Option<String>,
    /// `D`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
    #[uniffi(default = None)]
    pub timeframe: Option<String>,
    /// `asc` or `desc`.
    #[uniffi(default = None)]
    pub sort: Option<String>,
    /// Options only.
    #[uniffi(default = None)]
    pub strike_price: Option<f64>,
    /// Options only: `C` or `P`.
    #[uniffi(default = None)]
    pub call_put: Option<String>,
    /// `true` asks for the after-hours session (`session=afterhours`).
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
}

impl QueryParams for FutOptHistoricalCandlesParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self;
            from: Str, to: Str, contract_month: Str, fields: Str, timeframe: Str, sort: Str,
            strike_price: F64, call_put: Str, after_hours: Flag,
        )
    }
}

/// Parameters for `futopt/historical/daily`.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct FutOptDailyParams {
    /// `YYYY-MM-DD`.
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// `true` asks for the after-hours session (`session=afterhours`).
    #[uniffi(default = None)]
    pub after_hours: Option<bool>,
}

impl QueryParams for FutOptDailyParams {
    fn fields(&self) -> Vec<Field> {
        fields!(self; date: Str, after_hours: Flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketdata_core::rest::params::ENDPOINTS;
    use std::collections::BTreeSet;

    fn spec(path: &[&str]) -> &'static EndpointSpec {
        EndpointSpec::for_path(path).unwrap_or_else(|| panic!("{} is not in the table", path.join("/")))
    }

    fn query(path: &[&str], fields: Vec<Field>) -> Vec<Pair> {
        to_query(spec(path), fields).unwrap_or_else(|e| panic!("{}: {e}", path.join("/")))
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn assert_invalid_parameter(result: Result<Vec<Pair>, MarketDataError>, name: &str) {
        match result {
            Err(MarketDataError::ApiError { msg, info }) => {
                assert_eq!(info.code, marketdata_core::error_code::INVALID_PARAMETER);
                assert!(msg.starts_with(&format!("Invalid parameter '{name}':")), "{msg}");
            }
            other => panic!("expected INVALID_PARAMETER, got {other:?}"),
        }
    }

    // -- every record, fully set ------------------------------------------

    fn stock_tickers() -> StockTickersParams {
        StockTickersParams {
            exchange: s("TWSE"),
            market: s("TSE"),
            industry: s("24"),
            is_normal: Some(true),
            is_attention: Some(false),
            is_disposition: Some(false),
            is_halted: Some(false),
            symbol: s("23"),
        }
    }

    fn odd_lot() -> OddLotParams {
        OddLotParams { odd_lot: Some(true) }
    }

    fn stock_candles() -> StockCandlesParams {
        StockCandlesParams { timeframe: s("5"), odd_lot: Some(true), sort: s("asc") }
    }

    fn stock_trades() -> StockTradesParams {
        StockTradesParams {
            odd_lot: Some(true),
            offset: Some(10),
            limit: Some(5),
            sort: s("desc"),
            is_trial: Some(false),
        }
    }

    fn stock_historical_candles() -> StockHistoricalCandlesParams {
        StockHistoricalCandlesParams {
            from: s("2024-01-01"),
            to: s("2024-01-31"),
            timeframe: s("D"),
            fields: s("open,close"),
            sort: s("asc"),
            adjusted: Some(true),
        }
    }

    fn snapshot() -> SnapshotParams {
        SnapshotParams { type_filter: s("COMMONSTOCK") }
    }

    fn movers() -> MoversParams {
        MoversParams {
            type_filter: s("ALLBUT0999"),
            gt: Some(1.0),
            gte: Some(2.5),
            lt: Some(3.0),
            lte: Some(4.0),
            eq: Some(5.0),
        }
    }

    fn technical() -> TechnicalParams {
        TechnicalParams { from: s("2024-01-01"), to: s("2024-01-31"), timeframe: s("D") }
    }

    fn corporate_actions() -> CorporateActionsParams {
        CorporateActionsParams {
            start_date: s("2024-01-01"),
            end_date: s("2024-01-31"),
            exchange: s("TWSE"),
            sort: s("asc"),
        }
    }

    fn ownership() -> OwnershipParams {
        OwnershipParams { from: s("2024-01-01"), to: s("2024-01-31"), sort: s("desc") }
    }

    fn futopt_products() -> FutOptProductsParams {
        FutOptProductsParams {
            exchange: s("TAIFEX"),
            after_hours: Some(true),
            contract_type: s("I"),
            status: s("N"),
        }
    }

    fn futopt_tickers() -> FutOptTickersParams {
        FutOptTickersParams {
            exchange: s("TAIFEX"),
            after_hours: Some(true),
            product: s("TXF"),
            contract_type: s("I"),
            is_spread: Some(false),
        }
    }

    fn after_hours() -> AfterHoursParams {
        AfterHoursParams { after_hours: Some(true) }
    }

    fn futopt_candles() -> FutOptCandlesParams {
        FutOptCandlesParams { after_hours: Some(true), timeframe: s("1") }
    }

    fn futopt_trades() -> FutOptTradesParams {
        FutOptTradesParams { after_hours: Some(true), offset: Some(0), limit: Some(10), is_trial: Some(true) }
    }

    fn futopt_historical_candles() -> FutOptHistoricalCandlesParams {
        FutOptHistoricalCandlesParams {
            from: s("2024-01-01"),
            to: s("2024-01-31"),
            contract_month: s("202401"),
            fields: s("open,close"),
            timeframe: s("D"),
            sort: s("asc"),
            strike_price: Some(18000.0),
            call_put: s("C"),
            after_hours: Some(true),
        }
    }

    fn futopt_daily() -> FutOptDailyParams {
        FutOptDailyParams { date: s("2024-01-02"), after_hours: Some(true) }
    }

    fn positional(fields: &[(&'static str, &str)]) -> Vec<Field> {
        fields.iter().map(|(k, v)| (*k, Value::Str(v.to_string()))).collect()
    }

    /// Every endpoint: the client method's positional fields plus its record
    /// fully set, in `ENDPOINTS` order.
    fn full_requests() -> Vec<(&'static [&'static str], Vec<Field>)> {
        fn row(path: &'static [&'static str], positional: Vec<Field>, record: Vec<Field>) -> (&'static [&'static str], Vec<Field>) {
            let mut fields = positional;
            fields.extend(record);
            (path, fields)
        }
        let period = &[("period", "20")][..];
        vec![
            row(&["stock", "intraday", "tickers"], positional(&[("type", "EQUITY")]), stock_tickers().fields()),
            row(&["stock", "intraday", "ticker"], vec![], odd_lot().fields()),
            row(&["stock", "intraday", "quote"], vec![], odd_lot().fields()),
            row(&["stock", "intraday", "candles"], vec![], stock_candles().fields()),
            row(&["stock", "intraday", "trades"], vec![], stock_trades().fields()),
            row(&["stock", "intraday", "volumes"], vec![], odd_lot().fields()),
            row(&["stock", "historical", "candles"], vec![], stock_historical_candles().fields()),
            row(&["stock", "historical", "stats"], vec![], vec![]),
            row(&["stock", "snapshot", "quotes"], vec![], snapshot().fields()),
            row(
                &["stock", "snapshot", "movers"],
                positional(&[("direction", "up"), ("change", "percent")]),
                movers().fields(),
            ),
            row(&["stock", "snapshot", "actives"], positional(&[("trade", "volume")]), snapshot().fields()),
            row(&["stock", "technical", "sma"], positional(period), technical().fields()),
            row(&["stock", "technical", "rsi"], positional(period), technical().fields()),
            row(
                &["stock", "technical", "kdj"],
                positional(&[("r_period", "9"), ("k_period", "3"), ("d_period", "3")]),
                technical().fields(),
            ),
            row(
                &["stock", "technical", "macd"],
                positional(&[("fast", "12"), ("slow", "26"), ("signal", "9")]),
                technical().fields(),
            ),
            row(&["stock", "technical", "bb"], positional(period), technical().fields()),
            row(&["stock", "corporate-actions", "dividends"], vec![], corporate_actions().fields()),
            row(&["stock", "corporate-actions", "listing-applicants"], vec![], corporate_actions().fields()),
            row(
                &["stock", "corporate-actions", "capital-changes"],
                vec![],
                CorporateActionsParams { exchange: None, ..corporate_actions() }.fields(),
            ),
            row(&["stock", "ownership", "etf-holdings"], vec![], ownership().fields()),
            row(&["stock", "ownership", "institutional-trades"], vec![], ownership().fields()),
            row(&["stock", "ownership", "director-holdings"], vec![], ownership().fields()),
            row(&["stock", "ownership", "tdcc-distribution"], vec![], ownership().fields()),
            row(&["futopt", "intraday", "products"], positional(&[("type", "FUTURE")]), futopt_products().fields()),
            row(&["futopt", "intraday", "tickers"], positional(&[("type", "OPTION")]), futopt_tickers().fields()),
            row(&["futopt", "intraday", "ticker"], vec![], after_hours().fields()),
            row(&["futopt", "intraday", "quote"], vec![], after_hours().fields()),
            row(&["futopt", "intraday", "candles"], vec![], futopt_candles().fields()),
            row(&["futopt", "intraday", "trades"], vec![], futopt_trades().fields()),
            row(&["futopt", "intraday", "volumes"], vec![], after_hours().fields()),
            row(&["futopt", "historical", "candles"], vec![], futopt_historical_candles().fields()),
            row(&["futopt", "historical", "daily"], vec![], futopt_daily().fields()),
        ]
    }

    // (T1) The record for every endpoint covers the endpoint's whole table
    // entry, and nothing else.
    #[test]
    fn every_endpoint_record_matches_the_table() {
        let requests = full_requests();
        assert_eq!(requests.len(), ENDPOINTS.len(), "one row per endpoint");
        let rows: BTreeSet<_> = requests.iter().map(|(path, _)| *path).collect();
        assert_eq!(rows.len(), ENDPOINTS.len(), "duplicate row");
        for (path, fields) in requests {
            let spec = spec(path);
            let sent: BTreeSet<&str> = query(path, fields).into_iter().map(|(k, _)| k).collect();
            let table: BTreeSet<&str> = spec.names().collect();
            assert_eq!(sent, table, "{}", path.join("/"));
        }
    }

    #[test]
    fn capital_changes_rejects_exchange() {
        let path = &["stock", "corporate-actions", "capital-changes"];
        assert_invalid_parameter(to_query(spec(path), corporate_actions().fields()), "exchange");
        let sent: BTreeSet<&str> = query(path, CorporateActionsParams { exchange: None, ..corporate_actions() }.fields())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(sent, spec(path).names().collect());
    }

    // (T2) Flags send the table's literal, which differs per endpoint, and
    // are absent when false or unset.
    #[test]
    fn flags_send_the_table_literal() {
        assert_eq!(
            query(&["stock", "intraday", "trades"], stock_trades().fields()),
            vec![
                ("type", "oddlot".to_string()),
                ("offset", "10".to_string()),
                ("limit", "5".to_string()),
                ("sort", "desc".to_string()),
                ("isTrial", "false".to_string()),
            ]
        );
        assert_eq!(
            query(&["futopt", "intraday", "products"], FutOptProductsParams { after_hours: Some(true), ..Default::default() }.fields()),
            vec![("session", "AFTERHOURS".to_string())]
        );
        assert_eq!(
            query(&["futopt", "intraday", "tickers"], FutOptTickersParams { after_hours: Some(true), ..Default::default() }.fields()),
            vec![("session", "AFTERHOURS".to_string())]
        );
        assert_eq!(
            query(&["futopt", "intraday", "ticker"], after_hours().fields()),
            vec![("session", "afterhours".to_string())]
        );
        assert_eq!(
            query(&["futopt", "historical", "daily"], FutOptDailyParams { date: None, after_hours: Some(true) }.fields()),
            vec![("session", "afterhours".to_string())]
        );
        for value in [Some(false), None] {
            assert!(query(&["stock", "intraday", "quote"], OddLotParams { odd_lot: value }.fields()).is_empty());
            assert!(query(&["futopt", "intraday", "quote"], AfterHoursParams { after_hours: value }.fields()).is_empty());
            assert!(query(&["futopt", "intraday", "products"], FutOptProductsParams { after_hours: value, ..Default::default() }.fields()).is_empty());
        }
    }

    // (T3) Value formatting.
    #[test]
    fn values_are_formatted_for_the_wire() {
        let path = &["stock", "snapshot", "movers"];
        assert_eq!(
            query(path, MoversParams { gte: Some(2380.5), lte: Some(2380.0), ..Default::default() }.fields()),
            vec![("gte", "2380.5".to_string()), ("lte", "2380".to_string())]
        );
        assert_eq!(
            query(path, MoversParams { eq: Some(0.0), ..Default::default() }.fields()),
            vec![("eq", "0".to_string())]
        );
        assert_eq!(
            query(
                &["stock", "intraday", "tickers"],
                StockTickersParams { is_normal: Some(true), is_halted: Some(false), ..Default::default() }.fields()
            ),
            vec![("isNormal", "true".to_string()), ("isHalted", "false".to_string())]
        );
        // An empty string is still a value.
        assert_eq!(
            query(&["stock", "ownership", "etf-holdings"], OwnershipParams { sort: s(""), ..Default::default() }.fields()),
            vec![("sort", String::new())]
        );
        assert_eq!(
            query(&["futopt", "intraday", "trades"], FutOptTradesParams { offset: Some(0), ..Default::default() }.fields()),
            vec![("offset", "0".to_string())]
        );
    }

    #[test]
    fn default_records_send_nothing() {
        let empty: Vec<Field> = Vec::new();
        assert_eq!(fields_of::<StockTradesParams>(None), empty);
        assert_eq!(StockTradesParams::default().fields(), empty);
        assert_eq!(StockTickersParams::default().fields(), empty);
        assert_eq!(MoversParams::default().fields(), empty);
        assert_eq!(FutOptHistoricalCandlesParams::default().fields(), empty);
        assert!(query(&["stock", "historical", "stats"], Vec::new()).is_empty());
    }

    #[test]
    fn unknown_field_names_the_endpoint_and_the_accepted_keys() {
        let path = &["stock", "corporate-actions", "capital-changes"];
        match to_query(spec(path), corporate_actions().fields()) {
            Err(MarketDataError::ApiError { msg, .. }) => assert_eq!(
                msg,
                "Invalid parameter 'exchange': `stock.corporate-actions.capital-changes` \
                 does not accept `exchange`; accepted keys: start_date, end_date, sort"
            ),
            other => panic!("expected INVALID_PARAMETER, got {other:?}"),
        }
    }
}
