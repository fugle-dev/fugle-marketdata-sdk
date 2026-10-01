//! Shape checks for the objects JavaScript callers pass in (#294).
//!
//! `#[napi(object)]` reads only the fields it declares, so a mistyped key
//! (`reconnect: { maxRetries: 3 }`), a string where an object belongs
//! (`version: 'v1.0'`) or a number napi coerces (`messageBuffer: -1`) used to
//! pass silently. Each caller-facing object is described here by a table of
//! its fields; [`check`] walks the JS value against it before napi's own
//! conversion runs, and every refusal is a JS `TypeError` that says what was
//! wrong and what to write instead.
//!
//! An `undefined` value counts as not given, for known and unknown keys
//! alike, so `{ ...options, foo: undefined }` passes. Everything else
//! (`null` included) has to match its field.
//!
//! The value is first read into [`JsVal`], so the rules themselves are plain
//! Rust and unit-tested below; the jest suites cover the napi side.

use marketdata_core::websocket::version::option::{self as version_option, VersionOptionError};
use napi::bindgen_prelude::{FromNapiValue, TypeName, ValidateNapiValue};
use napi::{sys, ValueType};

/// A JS value, read as far as the checks need it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsVal {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    /// A typed array or Buffer; `true` when it is a `Uint8Array` (a Node
    /// `Buffer` is one).
    Bytes(bool),
    Array,
    Function,
    /// A plain object with its own enumerable string keys, in order. Read
    /// only to the depth the tables reach; deeper objects are left empty.
    Object(Vec<(String, JsVal)>),
    /// Anything else (symbol, bigint, external).
    Other(&'static str),
    /// A property whose getter threw. Any field accepts it: a legacy field
    /// is not read anyway (#262), and for a current one napi's conversion
    /// reads it again and throws the getter's own error, as before.
    Unreadable,
}

impl JsVal {
    /// The type as an error message names it.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Undefined => "undefined".into(),
            Self::Null => "null".into(),
            Self::Bool(_) => "boolean".into(),
            Self::Number(n) => format!("number {}", number_text(*n)),
            Self::String(_) => "string".into(),
            Self::Bytes(_) => "typed array".into(),
            Self::Array => "array".into(),
            Self::Function => "function".into(),
            Self::Object(_) => "object".into(),
            Self::Other(kind) => (*kind).into(),
            Self::Unreadable => "a value whose getter threw".into(),
        }
    }
}

/// JS spelling of a number: `NaN`, `Infinity`, `-1`, `1.5`.
fn number_text(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 { "Infinity".into() } else { "-Infinity".into() }
    } else {
        format!("{n}")
    }
}

/// What one field accepts.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind {
    Str,
    Bool,
    /// A non-negative integer that fits a `u32`.
    U32,
    /// A finite, non-negative number of milliseconds; fractions are kept.
    Millis,
    /// A `Uint8Array` / `Buffer`.
    Bytes,
    /// A nested object with its own table; `example` goes in the message
    /// when the value is not an object.
    Object { fields: &'static [Field], example: &'static str },
    /// The per-product streaming `version` map.
    Version,
    /// A `@fugle/marketdata` 1.x field that 3.0 no longer reads: accepted
    /// whatever its value, and warned about where it is read (#262).
    Legacy,
    /// Read by the other client (a shared 1.x-style options object): accepted
    /// whatever its value.
    Ignored,
}

/// One key of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Field {
    pub name: &'static str,
    pub kind: Kind,
}

const fn field(name: &'static str, kind: Kind) -> Field {
    Field { name, kind }
}

/// Names a table lists in an "unknown key" message: the current ones only.
fn known_names(fields: &[Field]) -> String {
    fields
        .iter()
        .filter(|f| !matches!(f.kind, Kind::Legacy | Kind::Ignored))
        .map(|f| f.name)
        .collect::<Vec<_>>()
        .join(", ")
}

const RECONNECT_FIELDS: &[Field] = &[
    field("enabled", Kind::Bool),
    field("maxAttempts", Kind::U32),
    field("initialDelayMs", Kind::Millis),
    field("maxDelayMs", Kind::Millis),
];

const HEALTH_CHECK_FIELDS: &[Field] = &[
    field("enabled", Kind::Bool),
    field("heartbeatTimeoutMs", Kind::Millis),
    field("probeEnabled", Kind::Bool),
    field("idleProbeAfterMs", Kind::Millis),
    field("probeTimeoutMs", Kind::Millis),
    field("pingInterval", Kind::Legacy),
    field("maxMissedPongs", Kind::Legacy),
];

/// `new WebSocketClient(options)`.
pub(crate) const WEBSOCKET_CLIENT_FIELDS: &[Field] = &[
    field("apiKey", Kind::Str),
    field("bearerToken", Kind::Str),
    field("sdkToken", Kind::Str),
    field("baseUrl", Kind::Str),
    field("version", Kind::Version),
    field("reconnect", Kind::Object { fields: RECONNECT_FIELDS, example: "{ enabled: false }" }),
    field("healthCheck", Kind::Object { fields: HEALTH_CHECK_FIELDS, example: "{ enabled: true }" }),
    field("tlsRootCertPem", Kind::Bytes),
    field("tlsAcceptInvalidCerts", Kind::Bool),
    field("messageOverflow", Kind::Str),
    field("messageBuffer", Kind::U32),
    field("authTimeoutMs", Kind::Millis),
];

/// `new RestClient(options)`. The WebSocket-only keys are accepted and not
/// read, so one options object can build both clients: 1.x documented
/// `version` (and took `healthCheck`) on the same object.
pub(crate) const REST_CLIENT_FIELDS: &[Field] = &[
    field("apiKey", Kind::Str),
    field("bearerToken", Kind::Str),
    field("sdkToken", Kind::Str),
    field("baseUrl", Kind::Str),
    field("tlsRootCertPem", Kind::Bytes),
    field("tlsAcceptInvalidCerts", Kind::Bool),
    field("version", Kind::Ignored),
    field("reconnect", Kind::Ignored),
    field("healthCheck", Kind::Ignored),
    field("messageOverflow", Kind::Ignored),
    field("messageBuffer", Kind::Ignored),
    field("authTimeoutMs", Kind::Ignored),
];

/// Check a constructor's options object. `owner` starts every message
/// (`WebSocketClient options`).
pub(crate) fn check_options(owner: &str, value: &JsVal, fields: &[Field]) -> Result<(), String> {
    if !matches!(value, JsVal::Object(_)) {
        let hint = match value {
            JsVal::String(_) => " To pass an API key, write { apiKey: '<key>' }.",
            _ => "",
        };
        return Err(format!(
            "{owner} must be an object like {{ apiKey: '<key>' }}, got {}.{hint}",
            value.describe()
        ));
    }
    check_fields(owner, "", value, fields)
}

/// Check every own key of `object` against `fields`; `path` prefixes nested
/// names (`reconnect.`).
pub(crate) fn check_fields(owner: &str, path: &str, object: &JsVal, fields: &[Field]) -> Result<(), String> {
    let JsVal::Object(entries) = object else {
        return Ok(());
    };
    for (key, value) in entries {
        if matches!(value, JsVal::Undefined) {
            continue;
        }
        let Some(spec) = fields.iter().find(|f| f.name == key) else {
            return Err(format!(
                "{owner}: unknown option '{path}{key}' (known: {})",
                known_names(fields)
            ));
        };
        check_value(owner, &format!("{path}{key}"), value, spec.kind)?;
    }
    Ok(())
}

fn check_value(owner: &str, name: &str, value: &JsVal, kind: Kind) -> Result<(), String> {
    let wrong = |expected: &str| Err(format!("{owner}: {name} must be {expected}, got {}", value.describe()));
    if matches!(value, JsVal::Unreadable) {
        return Ok(());
    }
    match kind {
        Kind::Legacy | Kind::Ignored => Ok(()),
        Kind::Str => match value {
            JsVal::String(_) => Ok(()),
            _ => wrong("a string"),
        },
        Kind::Bool => match value {
            JsVal::Bool(_) => Ok(()),
            _ => wrong("a boolean"),
        },
        Kind::U32 => match value {
            JsVal::Number(n) if n.is_finite() && n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u32::MAX) => Ok(()),
            _ => wrong("a non-negative integer"),
        },
        Kind::Millis => match value {
            JsVal::Number(n) if n.is_finite() && *n >= 0.0 => Ok(()),
            _ => wrong("a finite, non-negative number of milliseconds"),
        },
        Kind::Bytes => match value {
            JsVal::Bytes(true) => Ok(()),
            _ => wrong("a Uint8Array or Buffer holding PEM bytes"),
        },
        Kind::Object { fields, example } => match value {
            JsVal::Object(_) => check_fields(owner, &format!("{name}."), value, fields),
            _ => wrong(&format!("an object like {example}")),
        },
        Kind::Version => check_version(owner, value),
    }
}

/// The `version` map: products and versions are core's (#294); the bare
/// string message is 1.x's (`@fugle/marketdata` 1.7.0 threw the same
/// `TypeError` for it).
fn check_version(owner: &str, value: &JsVal) -> Result<(), String> {
    let entries = match value {
        JsVal::Object(entries) => entries,
        JsVal::String(bare) => return Err(format!("{owner}: {}", bare_version_message(bare))),
        other => {
            return Err(format!(
                "{owner}: version must be a per-product map like {{ futopt: 'v1.0' }}, got {}",
                other.describe()
            ))
        }
    };
    let mut given = Vec::new();
    for (product, requested) in entries {
        match requested {
            JsVal::Undefined => {}
            JsVal::String(v) => given.push((product.as_str(), v.as_str())),
            other => {
                // An unknown product is the clearer error, whatever its value.
                if !version_option::product_names().contains(&product.as_str()) {
                    return Err(format!("{owner}: {}", version_message(&VersionOptionError::UnknownProduct(product.clone()))));
                }
                return Err(format!(
                    "{owner}: version.{product} must be a version string, e.g. 'v1.1', got {}",
                    other.describe()
                ));
            }
        }
    }
    version_option::resolve(given).map(|_| ()).map_err(|err| format!("{owner}: {}", version_message(&err)))
}

/// A refused `version` entry, in this binding's wording.
pub(crate) fn version_message(err: &VersionOptionError) -> String {
    match err {
        VersionOptionError::UnknownProduct(key) => format!(
            "unknown product '{key}' in version map (known: {})",
            version_option::product_names().join(", ")
        ),
        VersionOptionError::Unsupported { product, .. } => format!(
            "{} Remove it from the version map to use {}.",
            err.describe(),
            version_option::default_for(product).unwrap_or("the default")
        ),
    }
}

/// `version: 'v1.0'`: name the map that asks for it on every product that
/// serves it.
fn bare_version_message(bare: &str) -> String {
    let products = version_option::products_serving(bare);
    let fix = if products.is_empty() {
        format!("No product serves {bare}.")
    } else {
        let pairs: Vec<String> = products.iter().map(|p| format!("{p}: '{bare}'")).collect();
        format!("Use version: {{ {} }}.", pairs.join(", "))
    };
    format!("version must be a per-product map, not the bare string '{bare}'. {fix}")
}

// ---------------------------------------------------------------------------
// Reading JS values

/// Read `value`, descending `depth` levels into objects.
///
/// # Safety
/// `env` and `value` must be live handles on the JS thread.
pub(crate) unsafe fn read(env: sys::napi_env, value: sys::napi_value, depth: usize) -> napi::Result<JsVal> {
    let mut kind = 0;
    napi::check_status!(unsafe { sys::napi_typeof(env, value, &mut kind) })?;
    Ok(match kind {
        sys::ValueType::napi_undefined => JsVal::Undefined,
        sys::ValueType::napi_null => JsVal::Null,
        sys::ValueType::napi_boolean => JsVal::Bool(unsafe { bool::from_napi_value(env, value)? }),
        sys::ValueType::napi_number => JsVal::Number(unsafe { f64::from_napi_value(env, value)? }),
        sys::ValueType::napi_string => JsVal::String(unsafe { String::from_napi_value(env, value)? }),
        sys::ValueType::napi_function => JsVal::Function,
        sys::ValueType::napi_object => unsafe { read_object(env, value, depth)? },
        sys::ValueType::napi_symbol => JsVal::Other("symbol"),
        sys::ValueType::napi_bigint => JsVal::Other("bigint"),
        _ => JsVal::Other("value"),
    })
}

unsafe fn read_object(env: sys::napi_env, value: sys::napi_value, depth: usize) -> napi::Result<JsVal> {
    let mut hit = false;
    napi::check_status!(unsafe { sys::napi_is_array(env, value, &mut hit) })?;
    if hit {
        return Ok(JsVal::Array);
    }
    napi::check_status!(unsafe { sys::napi_is_typedarray(env, value, &mut hit) })?;
    if hit {
        let mut array_type = 0;
        napi::check_status!(unsafe {
            sys::napi_get_typedarray_info(
                env,
                value,
                &mut array_type,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        })?;
        return Ok(JsVal::Bytes(array_type == sys::TypedarrayType::uint8_array));
    }
    napi::check_status!(unsafe { sys::napi_is_buffer(env, value, &mut hit) })?;
    if hit {
        return Ok(JsVal::Bytes(true));
    }
    if depth == 0 {
        return Ok(JsVal::Object(Vec::new()));
    }

    let mut keys = std::ptr::null_mut();
    napi::check_status!(unsafe {
        sys::napi_get_all_property_names(
            env,
            value,
            sys::KeyCollectionMode::own_only,
            sys::KeyFilter::enumerable | sys::KeyFilter::skip_symbols,
            sys::KeyConversion::numbers_to_strings,
            &mut keys,
        )
    })?;
    let mut len = 0;
    napi::check_status!(unsafe { sys::napi_get_array_length(env, keys, &mut len) })?;
    let mut entries = Vec::with_capacity(len as usize);
    for i in 0..len {
        let mut key = std::ptr::null_mut();
        napi::check_status!(unsafe { sys::napi_get_element(env, keys, i, &mut key) })?;
        let name = unsafe { String::from_napi_value(env, key)? };
        let mut item = std::ptr::null_mut();
        let read_item = if unsafe { sys::napi_get_property(env, value, key, &mut item) } == sys::Status::napi_ok {
            unsafe { read(env, item, depth - 1)? }
        } else {
            clear_exception(env);
            JsVal::Unreadable
        };
        entries.push((name, read_item));
    }
    Ok(JsVal::Object(entries))
}

/// Drop the exception a throwing getter left pending.
fn clear_exception(env: sys::napi_env) {
    let mut pending = false;
    if unsafe { sys::napi_is_exception_pending(env, &mut pending) } == sys::Status::napi_ok && pending {
        let mut ignored = std::ptr::null_mut();
        unsafe { sys::napi_get_and_clear_last_exception(env, &mut ignored) };
    }
}

/// Throw (or reject with) a JS `TypeError` carrying `message`.
pub(crate) fn type_error(env: sys::napi_env, message: &str) -> napi::Error {
    let raw = (|| {
        let mut text = std::ptr::null_mut();
        napi::check_status!(unsafe {
            sys::napi_create_string_utf8(env, message.as_ptr().cast(), message.len() as isize, &mut text)
        })?;
        let mut error = std::ptr::null_mut();
        napi::check_status!(unsafe { sys::napi_create_type_error(env, std::ptr::null_mut(), text, &mut error) })?;
        Ok::<_, napi::Error>(error)
    })();
    match raw {
        Ok(raw) => napi::Error::from(unsafe { napi::bindgen_prelude::Unknown::from_raw_unchecked(env, raw) }),
        Err(_) => napi::Error::new(napi::Status::InvalidArg, message.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Checked constructor arguments

/// The options object a constructor takes, with its table.
pub(crate) trait OptionsTable {
    const OWNER: &'static str;
    const FIELDS: &'static [Field];
}

impl OptionsTable for crate::websocket::WebSocketClientOptions {
    const OWNER: &'static str = "WebSocketClient options";
    const FIELDS: &'static [Field] = WEBSOCKET_CLIENT_FIELDS;
}

impl OptionsTable for crate::websocket::RestClientOptions {
    const OWNER: &'static str = "RestClient options";
    const FIELDS: &'static [Field] = REST_CLIENT_FIELDS;
}

/// `T`, converted only after [`check_options`] accepted the JS value.
pub struct Checked<T>(pub T);

impl<T: TypeName> TypeName for Checked<T> {
    fn type_name() -> &'static str {
        T::type_name()
    }

    fn value_type() -> ValueType {
        T::value_type()
    }
}

impl<T: ValidateNapiValue> ValidateNapiValue for Checked<T> {}

impl<T: FromNapiValue + OptionsTable> FromNapiValue for Checked<T> {
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
        // Two levels: the options and their nested objects.
        let value = unsafe { read(env, napi_val, 2)? };
        check_options(T::OWNER, &value, T::FIELDS).map_err(|message| type_error(env, &message))?;
        Ok(Self(unsafe { T::from_napi_value(env, napi_val)? }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(entries: &[(&str, JsVal)]) -> JsVal {
        JsVal::Object(entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
    }

    fn s(v: &str) -> JsVal {
        JsVal::String(v.into())
    }

    fn ws(entries: &[(&str, JsVal)]) -> Result<(), String> {
        let mut all = vec![("apiKey", s("k"))];
        all.extend(entries.iter().cloned());
        check_options("WebSocketClient options", &obj(&all), WEBSOCKET_CLIENT_FIELDS)
    }

    fn err(result: Result<(), String>) -> String {
        result.expect_err("expected a refusal")
    }

    #[test]
    fn test_options_must_be_object() {
        assert_eq!(
            err(check_options("WebSocketClient options", &JsVal::Undefined, WEBSOCKET_CLIENT_FIELDS)),
            "WebSocketClient options must be an object like { apiKey: '<key>' }, got undefined."
        );
        assert!(err(check_options("RestClient options", &s("k"), REST_CLIENT_FIELDS))
            .ends_with("got string. To pass an API key, write { apiKey: '<key>' }."));
        assert!(err(check_options("RestClient options", &JsVal::Array, REST_CLIENT_FIELDS)).contains("got array"));
    }

    #[test]
    fn test_unknown_keys() {
        assert_eq!(
            err(ws(&[("foo", JsVal::Number(1.0))])),
            "WebSocketClient options: unknown option 'foo' (known: apiKey, bearerToken, sdkToken, baseUrl, \
             version, reconnect, healthCheck, tlsRootCertPem, tlsAcceptInvalidCerts, messageOverflow, \
             messageBuffer, authTimeoutMs)"
        );
        assert_eq!(
            err(ws(&[("reconnect", obj(&[("maxRetries", JsVal::Number(3.0))]))])),
            "WebSocketClient options: unknown option 'reconnect.maxRetries' (known: enabled, maxAttempts, \
             initialDelayMs, maxDelayMs)"
        );
        // Legacy names are accepted but not advertised.
        assert_eq!(
            err(ws(&[("healthCheck", obj(&[("foo", JsVal::Null)]))])),
            "WebSocketClient options: unknown option 'healthCheck.foo' (known: enabled, heartbeatTimeoutMs, \
             probeEnabled, idleProbeAfterMs, probeTimeoutMs)"
        );
    }

    #[test]
    fn test_undefined_is_not_given() {
        assert!(ws(&[("foo", JsVal::Undefined), ("reconnect", JsVal::Undefined)]).is_ok());
        assert!(ws(&[("version", obj(&[("futopt", JsVal::Undefined)]))]).is_ok());
    }

    #[test]
    fn test_unreadable_is_left_to_napi() {
        assert!(ws(&[("healthCheck", obj(&[("pingInterval", JsVal::Unreadable), ("enabled", JsVal::Unreadable)]))]).is_ok());
        assert!(err(ws(&[("healthCheck", obj(&[("foo", JsVal::Unreadable)]))])).contains("unknown option 'healthCheck.foo'"));
    }

    #[test]
    fn test_legacy_health_check_fields_pass() {
        assert!(ws(&[("healthCheck", obj(&[("enabled", JsVal::Bool(true)), ("pingInterval", s("x")), ("maxMissedPongs", JsVal::Null)]))]).is_ok());
    }

    #[test]
    fn test_rest_accepts_websocket_keys() {
        let options = obj(&[
            ("apiKey", s("k")),
            ("version", s("v1.0")),
            ("healthCheck", JsVal::Null),
            ("reconnect", obj(&[("enabled", JsVal::Bool(false))])),
            ("messageBuffer", JsVal::Number(-1.0)),
        ]);
        assert!(check_options("RestClient options", &options, REST_CLIENT_FIELDS).is_ok());
        let unknown = obj(&[("apiKey", s("k")), ("foo", JsVal::Number(1.0))]);
        assert_eq!(
            err(check_options("RestClient options", &unknown, REST_CLIENT_FIELDS)),
            "RestClient options: unknown option 'foo' (known: apiKey, bearerToken, sdkToken, baseUrl, \
             tlsRootCertPem, tlsAcceptInvalidCerts)"
        );
    }

    #[test]
    fn test_rest_ignores_exactly_the_websocket_only_keys() {
        let rest: Vec<&str> = REST_CLIENT_FIELDS.iter().map(|f| f.name).collect();
        for f in WEBSOCKET_CLIENT_FIELDS {
            assert!(rest.contains(&f.name), "RestClient must accept {}", f.name);
        }
    }

    #[test]
    fn test_scalar_kinds() {
        assert_eq!(err(ws(&[("baseUrl", JsVal::Number(1.0))])), "WebSocketClient options: baseUrl must be a string, got number 1");
        assert_eq!(err(ws(&[("tlsAcceptInvalidCerts", JsVal::Null)])), "WebSocketClient options: tlsAcceptInvalidCerts must be a boolean, got null");
        for bad in [-1.0, 1.5, f64::NAN, f64::INFINITY, 4_294_967_296.0] {
            assert!(err(ws(&[("messageBuffer", JsVal::Number(bad))])).contains("messageBuffer must be a non-negative integer"), "{bad}");
        }
        assert!(ws(&[("messageBuffer", JsVal::Number(0.0))]).is_ok(), "0 is the constructor's own error");
        for bad in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(err(ws(&[("authTimeoutMs", JsVal::Number(bad))])).contains("authTimeoutMs must be a finite, non-negative number of milliseconds"), "{bad}");
        }
        assert!(ws(&[("authTimeoutMs", JsVal::Number(0.5))]).is_ok(), "the floor is core's");
        assert_eq!(
            err(ws(&[("reconnect", obj(&[("maxAttempts", JsVal::Number(f64::NAN))]))])),
            "WebSocketClient options: reconnect.maxAttempts must be a non-negative integer, got number NaN"
        );
        assert!(err(ws(&[("tlsRootCertPem", s("pem"))])).contains("tlsRootCertPem must be a Uint8Array or Buffer"));
        assert!(err(ws(&[("tlsRootCertPem", JsVal::Bytes(false))])).contains("got typed array"));
        assert!(ws(&[("tlsRootCertPem", JsVal::Bytes(true))]).is_ok());
    }

    #[test]
    fn test_nested_object_shape() {
        for (value, got) in [(s("x"), "string"), (JsVal::Bool(true), "boolean"), (JsVal::Array, "array"), (JsVal::Null, "null"), (JsVal::Function, "function")] {
            assert_eq!(
                err(ws(&[("reconnect", value.clone())])),
                format!("WebSocketClient options: reconnect must be an object like {{ enabled: false }}, got {got}")
            );
            assert!(err(ws(&[("healthCheck", value)])).contains("healthCheck must be an object like { enabled: true }"));
        }
    }

    #[test]
    fn test_version() {
        assert_eq!(
            err(ws(&[("version", s("v1.0"))])),
            "WebSocketClient options: version must be a per-product map, not the bare string 'v1.0'. \
             Use version: { stock: 'v1.0', futopt: 'v1.0' }."
        );
        assert!(err(ws(&[("version", s("v1.1"))])).ends_with("Use version: { futopt: 'v1.1' }."));
        assert!(err(ws(&[("version", s("v9"))])).ends_with("No product serves v9."));
        assert_eq!(
            err(ws(&[("version", JsVal::Null)])),
            "WebSocketClient options: version must be a per-product map like { futopt: 'v1.0' }, got null"
        );
        assert!(err(ws(&[("version", JsVal::Array)])).ends_with("got array"));
        assert_eq!(
            err(ws(&[("version", obj(&[("foo", s("v1.0"))]))])),
            "WebSocketClient options: unknown product 'foo' in version map (known: stock, futopt)"
        );
        assert!(err(ws(&[("version", obj(&[("foo", JsVal::Number(1.0))]))])).contains("unknown product 'foo'"));
        assert_eq!(
            err(ws(&[("version", obj(&[("futopt", JsVal::Null)]))])),
            "WebSocketClient options: version.futopt must be a version string, e.g. 'v1.1', got null"
        );
        assert_eq!(
            err(ws(&[("version", obj(&[("futopt", s("v9"))]))])),
            "WebSocketClient options: futopt streaming does not support v9 (supported: v1.0, v1.1). \
             Remove it from the version map to use v1.1."
        );
        assert!(ws(&[("version", obj(&[("futopt", s("v1.0")), ("stock", s("v1.0"))]))]).is_ok());
    }
}
