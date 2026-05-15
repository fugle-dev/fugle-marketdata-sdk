"""Deprecation shim — re-exports `fugle_marketdata` under the legacy
`marketdata_py` name for pre-3.0 local installs. Will be removed in 3.1.0.
"""
import warnings

warnings.warn(
    "`marketdata_py` is the pre-3.0 internal module name and is kept as a "
    "compatibility shim. Use `from fugle_marketdata import ...` instead. "
    "This shim will be removed in 3.1.0.",
    DeprecationWarning,
    stacklevel=2,
)

from fugle_marketdata import *  # noqa: F401,F403,E402
from fugle_marketdata import __all__  # noqa: E402
