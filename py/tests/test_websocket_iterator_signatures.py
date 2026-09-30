"""``MessageIterator`` and ``messages()`` match the type stub (#250).

The stub declared ``recv_timeout`` ``async`` where the method blocks and
returns its result, so code written to the stub awaited a dict; and it made
``timeout_ms`` of ``messages()`` keyword-only where the method takes it
positionally. Same check as ``test_rest_signatures.py`` does for REST: the
stub is parsed with ``ast`` and compared to ``inspect.signature`` of the
built module.

No method of ``MessageIterator`` is a coroutine function (``__anext__``
returns an awaitable built by the extension), so the stub may not spell any
of them ``async def``.
"""
import ast
import inspect
import pathlib

import pytest

import fugle_marketdata
from fugle_marketdata import MessageIterator, WebSocketClient

STUB = pathlib.Path(fugle_marketdata.__file__).with_name("__init__.pyi")

_KINDS = {
    inspect.Parameter.POSITIONAL_ONLY: "positional",
    inspect.Parameter.POSITIONAL_OR_KEYWORD: "positional",
    inspect.Parameter.VAR_POSITIONAL: "var_positional",
    inspect.Parameter.KEYWORD_ONLY: "keyword_only",
}


def _stub_functions(class_name):
    """``{method name: [def node, ...]}`` of a stub class; an overloaded
    method has one node per overload."""
    tree = ast.parse(STUB.read_text(), str(STUB))
    (cls,) = [n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == class_name]
    out = {}
    for fn in cls.body:
        if isinstance(fn, (ast.FunctionDef, ast.AsyncFunctionDef)):
            out.setdefault(fn.name, []).append(fn)
    return out


def _stub_params(fn):
    """``[(param name, kind), ...]`` of a stub def, ``self`` dropped."""
    a = fn.args
    params = [(arg.arg, "positional") for arg in a.posonlyargs + a.args if arg.arg != "self"]
    if a.vararg is not None:
        params.append((a.vararg.arg, "var_positional"))
    params += [(arg.arg, "keyword_only") for arg in a.kwonlyargs]
    return params


def _runtime_params(function):
    """``[(param name, kind), ...]`` of an unbound method, ``self`` dropped."""
    params = list(inspect.signature(function).parameters.values())[1:]
    return [(p.name, _KINDS[p.kind]) for p in params if p.kind is not inspect.Parameter.VAR_KEYWORD]


def _public_methods(cls):
    return sorted(n for n in dir(cls) if not n.startswith("_") and callable(getattr(cls, n)))


ITERATOR_STUB = _stub_functions("MessageIterator")
ITERATOR_METHODS = _public_methods(MessageIterator)


def _product_classes():
    # Nothing connects: the clients are only asked for their type.
    ws = WebSocketClient(api_key="test-key", base_url="ws://127.0.0.1:9")
    return [type(ws.stock), type(ws.futopt)]


PRODUCT_CLASSES = _product_classes()


def test_reflection_reaches_the_iterator_methods():
    assert {"try_recv", "recv_timeout"} <= set(ITERATOR_METHODS)
    assert [cls.__name__ for cls in PRODUCT_CLASSES] == ["StockWebSocketClient", "FutOptWebSocketClient"]


@pytest.mark.parametrize("name", ITERATOR_METHODS)
def test_iterator_method_matches_stub(name):
    assert name in ITERATOR_STUB, f"MessageIterator.{name} is not in __init__.pyi"
    (fn,) = ITERATOR_STUB[name]
    assert _stub_params(fn) == _runtime_params(getattr(MessageIterator, name))


def test_stub_declares_no_iterator_method_the_module_lacks():
    stray = [n for n in ITERATOR_STUB if not hasattr(MessageIterator, n)]
    assert stray == []


@pytest.mark.parametrize("name", sorted(ITERATOR_STUB))
def test_iterator_stub_is_async_only_where_the_method_is_a_coroutine_function(name):
    (fn,) = ITERATOR_STUB[name]
    declared_async = isinstance(fn, ast.AsyncFunctionDef)
    assert declared_async == inspect.iscoroutinefunction(getattr(MessageIterator, name))


@pytest.mark.parametrize("cls", PRODUCT_CLASSES, ids=lambda cls: cls.__name__)
def test_messages_matches_every_stub_overload(cls):
    overloads = _stub_functions(cls.__name__)["messages"]
    runtime = _runtime_params(cls.messages)
    assert runtime == [("timeout_ms", "positional"), ("raw", "keyword_only")]
    assert len(overloads) == 3
    for fn in overloads:
        assert not isinstance(fn, ast.AsyncFunctionDef)
        assert _stub_params(fn) == runtime
