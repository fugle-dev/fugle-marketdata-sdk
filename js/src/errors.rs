//! Errors handed to JavaScript.
//!
//! Every SDK error reaches JavaScript as a plain `Error` carrying the fields
//! of core's [`ErrorInfo`] (see `docs/errors.md`):
//!
//! | property | type |
//! |---|---|
//! | `message` | `string` |
//! | `code` | `number` |
//! | `sourceKind` | `'network' \| 'protocol' \| 'auth' \| 'rate_limit' \| 'client'` |
//! | `status` | `number \| null` |
//! | `body` | `string \| null` |
//! | `requestId` | `string \| null` |
//! | `headers` | `Record<string, string>` |
//!
//! The object has to be created on the JS thread, so async methods return a
//! [`Settled`] whose conversion builds it and rejects the promise with it.

use marketdata_core::{ErrorInfo, MarketDataError};
use napi::bindgen_prelude::{JsObjectValue, Object, ToNapiValue, TypeName, Unknown, ValueType};
use napi::{sys, Env, JsValue};
use serde_json::Value;

/// `new Error(info.message)` with the unified fields set.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn error_value(env: sys::napi_env, info: &ErrorInfo) -> napi::Result<sys::napi_value> {
    let env_ref = Env::from_raw(env);
    let message = env_ref.create_string(&info.message)?;
    let mut raw = std::ptr::null_mut();
    napi::check_status!(unsafe {
        sys::napi_create_error(env, std::ptr::null_mut(), message.raw(), &mut raw)
    })?;
    let mut error = Object::from_raw(env, raw);
    error.set_named_property("code", info.code)?;
    error.set_named_property("sourceKind", info.source_kind.as_str())?;
    error.set_named_property("status", info.status.map(u32::from))?;
    error.set_named_property("body", info.body.as_deref())?;
    error.set_named_property("requestId", info.request_id.as_deref())?;
    error.set_named_property("headers", info.headers.clone())?;
    Ok(raw)
}

/// A `napi::Error` that throws (or rejects with) [`error_value`] for `info`.
///
/// Must be called on the JS thread. Falls back to a plain message if the
/// object cannot be built.
pub fn js_error(env: &Env, info: &ErrorInfo) -> napi::Error {
    match error_value(env.raw(), info) {
        Ok(raw) => napi::Error::from(unsafe { Unknown::from_raw_unchecked(env.raw(), raw) }),
        Err(_) => napi::Error::from_reason(info.message.clone()),
    }
}

/// [`js_error`] for a core error.
pub fn to_napi_error(env: &Env, err: MarketDataError) -> napi::Error {
    js_error(env, &err.info())
}

/// A constructor failure: argument validation, or a core error that becomes
/// a JS error with the unified fields.
#[derive(Debug)]
pub enum BuildError {
    Napi(napi::Error),
    Core(MarketDataError),
}

impl From<napi::Error> for BuildError {
    fn from(err: napi::Error) -> Self {
        Self::Napi(err)
    }
}

impl From<MarketDataError> for BuildError {
    fn from(err: MarketDataError) -> Self {
        Self::Core(err)
    }
}

impl BuildError {
    /// The error to throw; call on the JS thread.
    pub fn into_napi(self, env: &Env) -> napi::Error {
        match self {
            Self::Napi(err) => err,
            Self::Core(err) => to_napi_error(env, err),
        }
    }
}

/// The outcome of a REST call, converted on the JS thread: the JSON value,
/// or a rejection with [`error_value`].
pub struct Settled(pub Result<Value, MarketDataError>);

impl TypeName for Settled {
    fn type_name() -> &'static str {
        "unknown"
    }

    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

impl ToNapiValue for Settled {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        match val.0 {
            Ok(value) => unsafe { Value::to_napi_value(env, value) },
            Err(err) => Err(js_error(&Env::from_raw(env), &err.info())),
        }
    }
}

/// Result type alias for NAPI operations
pub type NapiResult<T> = napi::Result<T>;

#[cfg(test)]
mod tests {
    use marketdata_core::{error_code, ErrorKind, HttpErrorContext, MarketDataError};

    // Building the JS object needs a live env; the jest suite covers that.
    // Here we pin what the object is built from.
    #[test]
    fn api_error_info_has_http_fields_and_no_code_prefix() {
        let err = MarketDataError::ApiError {
            status: 404,
            message: "not found".to_string(),
            http: Some(Box::new(HttpErrorContext::new(
                404,
                Some("not found".to_string()),
                [("x-request-id", "r1")],
            ))),
        };
        let info = err.info();
        assert_eq!(info.code, error_code::API);
        assert_eq!(info.source_kind, ErrorKind::Client);
        assert_eq!(info.message, "API error (status 404): not found");
        assert_eq!(info.status, Some(404));
        assert_eq!(info.request_id.as_deref(), Some("r1"));
    }
}
