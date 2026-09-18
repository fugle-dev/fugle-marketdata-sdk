//! The `**kwargs` of a REST method, resolved through core's parameter table.
//!
//! Every REST method has typed keyword arguments (the 3.x snake_case names)
//! plus `**_extra`. Whatever lands in `_extra` is either another spelling of
//! a typed argument or a mistake, and both used to end the same way: a
//! `UserWarning` and the key dropped, so `trades(symbol, limit=5)` returned
//! 50 trades and `ticker(symbol, type="oddlot")` returned board-lot data
//! (#164, #165).
//!
//! [`Kwargs::parse`] now maps each extra key onto the table in
//! `marketdata_core::rest::params` — the API's own names (`isTrial`, `from`,
//! `type="oddlot"`, `session="afterhours"`), which is what the 2.x SDK and the
//! developer.fugle.tw examples use, plus the 2.x `from_` alias for the
//! reserved word — and raises `TypeError` for anything else. Each method then
//! merges the resolved values into its typed arguments with the `take_*`
//! methods, which raise `TypeError` when the same parameter arrived under two
//! spellings. Values are passed through as given; only the two boolean flags
//! (`type="oddlot"`, `session="afterhours"`) are interpreted, because the
//! typed arguments behind them are booleans. Those are `Option<bool>` so that
//! "not given" is distinct from `False` and the conflict check covers them
//! like every other parameter.

use std::collections::HashMap;

use marketdata_core::rest::params::{EndpointSpec, ParamSpec};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// The 3.x keyword for a table parameter, where the signature spells it
/// differently: `from_date` / `to_date` for the reserved word and its pair,
/// and `type_filter` on the snapshot endpoints (whose builder method is
/// `type_filter`; the list endpoints take `type` as is). These are the
/// binding's own names, so they live here and not in the table.
fn py_name(spec: &ParamSpec) -> &'static str {
    match spec.canonical {
        "from" => "from_date",
        "to" => "to_date",
        "type" if spec.methods.contains(&"type_filter") => "type_filter",
        other => other,
    }
}

/// One resolved extra keyword: the spelling the caller used and its value.
struct Given<'py> {
    key: String,
    /// The key was the flag spelling (`odd_lot`, `oddLot`, `after_hours`),
    /// so the value is a boolean; otherwise it was the wire name (`type`,
    /// `session`) and the value is the literal the API takes.
    as_flag: bool,
    value: Bound<'py, PyAny>,
}

/// `**_extra` of one call, keyed by the table's canonical name.
pub struct Kwargs<'py> {
    method: &'static str,
    spec: &'static EndpointSpec,
    given: HashMap<&'static str, Given<'py>>,
}

impl<'py> Kwargs<'py> {
    /// Resolve every key of `extra`. `method` is the dotted name the table
    /// knows (`stock.intraday.trades`) and prefixes every error.
    pub fn parse(method: &'static str, extra: &Option<Bound<'py, PyDict>>) -> PyResult<Self> {
        let spec = EndpointSpec::for_name(method)
            .unwrap_or_else(|| panic!("{method} has no entry in core::rest::params"));
        let mut given: HashMap<&'static str, Given<'py>> = HashMap::new();
        let Some(extra) = extra else {
            return Ok(Self { method, spec, given });
        };
        for (key, value) in extra.iter() {
            let key: String = key.extract()?;
            let resolved = spec
                .resolve(&key)
                // 2.x accepted `from_` for the reserved word; the table only
                // records that `from` is a keyword, the trailing underscore
                // is this binding's rule.
                .or_else(|| {
                    let stem = key.strip_suffix('_')?;
                    spec.resolve(stem).filter(|r| r.spec.python_keyword)
                })
                .ok_or_else(|| unknown(method, spec, &key))?;
            let canonical = resolved.spec.canonical;
            if let Some(earlier) = given.get(canonical) {
                return Err(multiple(method, py_name(resolved.spec), &earlier.key, &key));
            }
            given.insert(canonical, Given { key, as_flag: resolved.as_flag, value });
        }
        Ok(Self { method, spec, given })
    }

    /// Merge into a typed `Option<String>` argument.
    pub fn take_string(&mut self, canonical: &str, typed: Option<String>) -> PyResult<Option<String>> {
        self.take(canonical, typed)
    }

    /// Merge into a typed `Option<T>` argument. A value of the wrong type
    /// fails the way the typed keyword would (pyo3's own message), prefixed
    /// with the method and the spelling the caller used.
    pub fn take<T>(&mut self, canonical: &str, typed: Option<T>) -> PyResult<Option<T>>
    where
        T: for<'a> FromPyObject<'a, 'py>,
    {
        let Some(given) = self.given.remove(canonical) else { return Ok(typed) };
        self.no_conflict(canonical, typed.is_some(), &given.key)?;
        if given.value.is_none() {
            return Ok(None);
        }
        extract::<T>(&given.value)
            .map(Some)
            .map_err(|err| self.bad_value(&given.key, &given.value, err))
    }

    /// Merge into a typed boolean flag (`odd_lot`, `after_hours`), `None`
    /// when the keyword was not given.
    ///
    /// The flag spelling takes a boolean. The wire spelling takes the literal
    /// the API documents: `type="oddlot"` exactly, since the server is
    /// case-sensitive there; `session` case-insensitively, since the server
    /// is, with `"regular"` meaning off.
    pub fn take_flag(&mut self, canonical: &str, typed: Option<bool>) -> PyResult<Option<bool>> {
        let Some(given) = self.given.remove(canonical) else { return Ok(typed) };
        self.no_conflict(canonical, typed.is_some(), &given.key)?;
        if given.value.is_none() {
            return Ok(None);
        }
        let spec = self.param(canonical);
        let literal = spec.flag.expect("take_flag is called for a flag parameter");
        if given.as_flag {
            return given
                .value
                .extract::<bool>()
                .map(Some)
                .map_err(|err| self.bad_value(&given.key, &given.value, err));
        }
        let value: String = given.value.extract().map_err(|err| self.bad_value(&given.key, &given.value, err))?;
        let on = match spec.name {
            "session" => {
                if value.eq_ignore_ascii_case("regular") {
                    return Ok(Some(false));
                }
                value.eq_ignore_ascii_case(literal)
            }
            _ => value == literal,
        };
        if on {
            Ok(Some(true))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(format!(
                "{}() argument '{}' must be '{literal}' (got '{value}'); the typed keyword is {}=True",
                self.method, given.key, canonical
            )))
        }
    }

    /// Everything resolved must have been taken by the method; a leftover
    /// means the table lists a parameter this method has no keyword for.
    pub fn finish(self) -> PyResult<()> {
        match self.given.into_values().next() {
            None => Ok(()),
            Some(given) => Err(PyTypeError::new_err(format!(
                "{}() does not support '{}' yet",
                self.method, given.key
            ))),
        }
    }

    fn bad_value(&self, key: &str, value: &Bound<'py, PyAny>, err: PyErr) -> PyErr {
        PyTypeError::new_err(format!("{}() argument '{key}': {}", self.method, err.value(value.py())))
    }

    fn no_conflict(&self, canonical: &str, typed_given: bool, key: &str) -> PyResult<()> {
        if typed_given {
            let spec = self.param(canonical);
            return Err(multiple(self.method, py_name(spec), py_name(spec), key));
        }
        Ok(())
    }

    fn param(&self, canonical: &str) -> &'static ParamSpec {
        self.spec
            .params
            .iter()
            .find(|p| p.canonical == canonical)
            .unwrap_or_else(|| panic!("{}: `{canonical}` is not in the table", self.method))
    }
}

/// `TypeError` for a key the endpoint does not take: names the method, the
/// nearest accepted spelling when there is one (a spelling that works as
/// written — `oddlot` points at `oddLot`, not at `type`, and `From` at
/// `from_`, not at the reserved word), and every accepted keyword.
fn unknown(method: &str, spec: &EndpointSpec, key: &str) -> PyErr {
    let hint = match spec.suggest(key) {
        Some(name) => {
            let keyword = spec.resolve(name).is_some_and(|r| r.spec.python_keyword && r.spec.name == name);
            let name = if keyword { format!("{name}_") } else { name.to_string() };
            format!(" Did you mean '{name}'?")
        }
        None => String::new(),
    };
    let mut accepted: Vec<String> = Vec::new();
    for p in spec.params {
        let typed = py_name(p);
        accepted.push(typed.to_string());
        if p.name != typed {
            accepted.push(p.name.to_string());
        }
        if p.python_keyword {
            accepted.push(format!("{}_", p.name));
        }
        accepted.extend(p.aliases.iter().map(|a| a.to_string()));
    }
    PyTypeError::new_err(format!(
        "{method}() got an unexpected keyword argument '{key}'.{hint} Accepted: {}",
        accepted.join(", ")
    ))
}

/// `extract` with the error already a `PyErr`; a plain closure cannot name
/// the associated error type under the `for<'a>` bound.
fn extract<'a, 'py, T: FromPyObject<'a, 'py>>(value: &'a Bound<'py, PyAny>) -> PyResult<T> {
    value.extract::<T>().map_err(Into::into)
}

fn multiple(method: &str, canonical: &str, first: &str, second: &str) -> PyErr {
    // Same wording as the ownership methods have used since 3.0.0-rc.1.
    let spellings = if first == canonical {
        format!("also passed as '{second}'")
    } else {
        format!("passed as '{first}' and '{second}'")
    };
    PyTypeError::new_err(format!(
        "{method}() got multiple values for {canonical} ({spellings})"
    ))
}
