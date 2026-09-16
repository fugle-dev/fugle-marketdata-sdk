//! Conversion from the server's JSON response into native Python values.
//!
//! REST responses reach this module as `serde_json::Value` exactly as the
//! server sent them — no intermediate struct, so no field is dropped and none
//! is invented. Objects become `dict`, arrays become `list`, and JSON scalars
//! become their Python counterparts.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

/// Recursively convert a JSON value into the equivalent Python object.
pub fn json_value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObject;

    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().unbind().into_any()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.to_owned().unbind().into_any())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.to_owned().unbind().into_any())
            } else {
                Ok(n.to_string().into_pyobject(py)?.to_owned().unbind().into_any())
            }
        }
        Value::String(s) => Ok(s.into_pyobject(py)?.to_owned().unbind().into_any()),
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py(py, item)?)?;
            }
            Ok(list.unbind().into_any())
        }
        Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (key, val) in obj {
                dict.set_item(key, json_value_to_py(py, val)?)?;
            }
            Ok(dict.unbind().into_any())
        }
    }
}

/// Convert a JSON response body into a Python `dict`.
///
/// # Errors
/// Returns `TypeError` if the endpoint returned something other than a JSON
/// object. Endpoints that return a top-level array are served by
/// [`json_value_to_py`] instead.
pub fn value_to_dict(py: Python<'_>, value: &Value) -> PyResult<Py<PyDict>> {
    let py_any = json_value_to_py(py, value)?;
    let bound = py_any.bind(py);
    let dict: &Bound<'_, PyDict> = bound.cast().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err("Expected a JSON object from the API")
    })?;
    Ok(dict.clone().unbind())
}
