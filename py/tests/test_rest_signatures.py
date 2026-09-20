"""The REST methods' Python signatures match the type stub (#217).

Every REST method takes its path parameter (``symbol``, ``market``,
``type``) and the query parameters core's ``rest::params`` table marks
``required(...)`` positionally; everything else is keyword-only. Before,
the implementation accepted every parameter positionally while the stub said
keyword-only for most of them, so ``candles("2330", "5")`` ran but failed
mypy, and ``historical.candles("2330", "2024-01-01", "D")`` silently sent the
date as ``to`` (seven optional strings in a row: the wrong slot is not an
error anywhere).

The method list comes from the module itself: every public callable reached
through the ``RestClient`` property tree. The stub is parsed with ``ast``.
The ``**_extra`` catch-all that carries the API's own spellings
(``isTrial``, ``from_``) is a runtime alias mechanism the stub does not
declare, so ``VAR_KEYWORD`` is left out of the comparison.
"""
import ast
import builtins
import inspect
import pathlib

import pytest

import fugle_marketdata
from fugle_marketdata import RestClient

STUB = pathlib.Path(fugle_marketdata.__file__).with_name("__init__.pyi")


def _rest_methods():
    """``(class name, method name, bound method)`` for every public callable
    on the clients reachable from ``RestClient`` through its properties."""
    # A closed local port, so the alias probe below never leaves the machine.
    root = RestClient(api_key="test-key", base_url="http://127.0.0.1:9")
    seen = set()
    found = []
    stack = [root]
    while stack:
        obj = stack.pop()
        cls = type(obj)
        if cls in seen:
            continue
        seen.add(cls)
        for name in sorted(dir(cls)):
            if name.startswith("_"):
                continue
            attr = getattr(cls, name)
            if inspect.isgetsetdescriptor(attr):
                value = getattr(obj, name)
                # pyo3 classes report ``builtins`` as their module too, so
                # tell a sub-client from ``base_url``'s str by the type name.
                if not hasattr(builtins, type(value).__name__):
                    stack.append(value)
            elif callable(attr):
                found.append((cls.__name__, name, getattr(obj, name)))
    return found


def _stub_signatures():
    """``{(class name, method name): [(param name, kind, has default), ...]}``
    from the stub, ``self`` dropped and ``**kwargs`` ignored."""
    tree = ast.parse(STUB.read_text(), str(STUB))
    out = {}
    for node in tree.body:
        if not isinstance(node, ast.ClassDef):
            continue
        for fn in node.body:
            if not isinstance(fn, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            if any(isinstance(d, ast.Name) and d.id == "property" for d in fn.decorator_list):
                continue  # a sub-client accessor, walked above, not a method
            a = fn.args
            params = []
            positional = a.posonlyargs + a.args
            defaults = [None] * (len(positional) - len(a.defaults)) + list(a.defaults)
            for arg, default in zip(positional, defaults):
                if arg.arg == "self":
                    continue
                params.append((arg.arg, "positional", default is not None))
            if a.vararg is not None:
                params.append((a.vararg.arg, "var_positional", False))
            for arg, default in zip(a.kwonlyargs, a.kw_defaults):
                params.append((arg.arg, "keyword_only", default is not None))
            out[(node.name, fn.name)] = params
    return out


def _runtime_signature(method):
    params = []
    for p in inspect.signature(method).parameters.values():
        if p.kind is inspect.Parameter.VAR_KEYWORD:
            continue
        kind = {
            inspect.Parameter.POSITIONAL_ONLY: "positional",
            inspect.Parameter.POSITIONAL_OR_KEYWORD: "positional",
            inspect.Parameter.VAR_POSITIONAL: "var_positional",
            inspect.Parameter.KEYWORD_ONLY: "keyword_only",
        }[p.kind]
        params.append((p.name, kind, p.default is not inspect.Parameter.empty))
    return params


METHODS = _rest_methods()
STUBS = _stub_signatures()


def test_reflection_reaches_every_rest_client():
    classes = {cls for cls, _, _ in METHODS}
    assert {"StockIntradayClient", "StockHistoricalClient", "StockSnapshotClient",
            "StockTechnicalClient", "StockOwnershipClient",
            "StockCorporateActionsClient", "FutOptIntradayClient",
            "FutOptHistoricalClient"} <= classes
    assert len(METHODS) > 60


@pytest.mark.parametrize(
    "cls, name, method",
    METHODS,
    ids=[f"{cls}.{name}" for cls, name, _ in METHODS],
)
def test_implementation_matches_stub(cls, name, method):
    assert (cls, name) in STUBS, f"{cls}.{name} is not in __init__.pyi"
    runtime = _runtime_signature(method)
    stub = STUBS[(cls, name)]
    # The stub may spell out keyword-only aliases the runtime takes through
    # ``**_extra`` (ownership's ``from_``/``to``); each must bind there.
    declared_only = [p for p in stub if p not in runtime]
    assert [p for p in stub if p not in declared_only] == runtime
    for alias, kind, _ in declared_only:
        assert kind == "keyword_only", (alias, kind)
        assert _accepts_keyword(method, runtime, alias), f"{alias} is not a runtime alias"


def _accepts_keyword(method, runtime, keyword):
    """Whether ``method(<positional>, keyword=...)`` gets past argument
    parsing: the keyword layer raises ``TypeError`` for an unknown name before
    any request (or event loop) is touched."""
    positional = [n for n, kind, has_default in runtime if kind == "positional" and not has_default]
    try:
        method(*["x"] * len(positional), **{keyword: "2024-01-01"})
    except TypeError:
        return False
    except Exception:
        return True
    return True


def test_stub_declares_nothing_the_module_lacks():
    classes = {cls for cls, _, _ in METHODS}
    implemented = {(cls, name) for cls, name, _ in METHODS}
    stray = [k for k in STUBS if k[0] in classes and k not in implemented and not k[1].startswith("_")]
    assert stray == []


@pytest.mark.parametrize(
    "cls, name, method",
    METHODS,
    ids=[f"{cls}.{name}" for cls, name, _ in METHODS],
)
def test_optional_parameters_are_keyword_only(cls, name, method):
    """No optional parameter sits in a positional slot unless core marks it
    required: the wrong positional slot would otherwise not be an error.
    ``from_core`` mirrors the ``required(...)`` entries of
    ``core/src/rest/params.rs``."""
    from_core = {
        ("StockSnapshotClient", "movers"): {"direction", "change"},
        ("StockSnapshotClient", "actives"): {"trade"},
        ("StockTechnicalClient", "sma"): {"period"},
        ("StockTechnicalClient", "rsi"): {"period"},
        ("StockTechnicalClient", "bb"): {"period"},
        ("StockTechnicalClient", "kdj"): {"r_period", "k_period", "d_period"},
        ("StockTechnicalClient", "macd"): {"fast", "slow", "signal"},
    }
    base = name[: -len("_async")] if name.endswith("_async") else name
    allowed = from_core.get((cls, base), set())
    positional_with_default = [
        n for n, kind, has_default in _runtime_signature(method)
        if kind == "positional" and has_default
    ]
    assert set(positional_with_default) <= allowed, positional_with_default


class TestCallShapes:
    """The shapes the issue lists, against a client that never reaches the
    network (the calls below raise ``TypeError`` before any request)."""

    @pytest.fixture
    def client(self):
        return RestClient(api_key="test-key", base_url="http://127.0.0.1:9")

    def test_positional_optional_is_a_type_error(self, client):
        with pytest.raises(TypeError, match="positional"):
            client.stock.intraday.candles("2330", "5")
        with pytest.raises(TypeError, match="positional"):
            client.stock.intraday.candles_async("2330", "5")
        with pytest.raises(TypeError, match="positional"):
            client.stock.historical.candles("2330", "2024-01-01", "2024-01-31")
        with pytest.raises(TypeError, match="positional"):
            client.stock.corporate_actions.dividends("2024-01-01")

    def test_keyword_forms_still_bind(self, client):
        for call in (
            lambda: client.stock.intraday.candles("2330", timeframe="5"),
            lambda: client.stock.intraday.candles(symbol="2330", timeframe="5"),
            lambda: client.stock.intraday.trades("2330", isTrial=True, limit=3),
            lambda: client.stock.technical.sma("2330", from_="2026-08-01", period=5),
        ):
            with pytest.raises(fugle_marketdata.MarketDataError):
                call()

    def test_required_query_parameters_stay_positional(self, client):
        for call in (
            lambda: client.stock.snapshot.movers("TSE", "up", "percent"),
            lambda: client.stock.snapshot.actives("TSE", "volume"),
            lambda: client.stock.technical.sma("2330", 5),
            lambda: client.stock.technical.kdj("2330", 9, 3, 3),
            lambda: client.stock.technical.macd("2330", 12, 26, 9),
            lambda: client.stock.intraday.tickers("EQUITY"),
        ):
            with pytest.raises(fugle_marketdata.MarketDataError):
                call()
        with pytest.raises(TypeError, match="positional"):
            client.stock.snapshot.movers("TSE", "up", "percent", "COMMONSTOCK")
