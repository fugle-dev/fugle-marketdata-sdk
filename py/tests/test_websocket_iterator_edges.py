"""Run-time edges of ``MessageIterator`` against a loopback server (#260).

``recv_timeout`` must let signal handlers run while it waits and accept any
``u64`` of milliseconds. A cancelled ``__anext__`` must not take a message:
its blocking wait used to outlive the cancellation by up to 100 ms and drop
whatever arrived meanwhile. An ``__anext__`` left pending when its event loop
closes must not print a traceback once the connection ends.
"""
import asyncio
import os
import signal
import statistics
import subprocess
import sys
import threading
import time

import pytest

from tests.ws_loopback import LoopbackServer, disconnect_quietly, product_ws

# A hang inside native code never lets the default signal-based timeout fire.
hard_timeout = pytest.mark.timeout(30, method="thread")

PY_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SUBSCRIPTION = {"channel": "trades", "symbol": "2330"}


@pytest.fixture
def server():
    with LoopbackServer() as srv:
        yield srv


class _Interrupted(Exception):
    pass


@hard_timeout
@pytest.mark.skipif(not hasattr(signal, "SIGUSR1"), reason="needs SIGUSR1")
def test_recv_timeout_wait_lets_signal_handlers_interrupt_it(server):
    # Like Ctrl+C: the handler must run during the wait, not once it times out.
    ws = product_ws(server.url, "stock")

    def interrupt(signum, frame):
        raise _Interrupted()

    previous = signal.signal(signal.SIGUSR1, interrupt)
    kill = threading.Timer(0.3, os.kill, args=(os.getpid(), signal.SIGUSR1))
    try:
        ws.connect()
        messages = ws.messages()
        assert next(messages)["event"] == "authenticated"
        kill.start()
        started = time.monotonic()
        with pytest.raises(_Interrupted):
            messages.recv_timeout(5000)
        assert time.monotonic() - started < 2, "the signal was only handled once the wait timed out"
        # The interrupted call took nothing: the next read starts at the ack.
        ws.subscribe(SUBSCRIPTION)
        assert messages.recv_timeout(5000)["event"] == "subscribed"
    finally:
        # Never restore the default handler while our signal may still come:
        # SIGUSR1's default action ends the process.
        kill.cancel()
        kill.join()
        try:
            time.sleep(0.1)  # a signal sent just before cancel() runs its handler now
        except _Interrupted:
            pass
        signal.signal(signal.SIGUSR1, previous)
        disconnect_quietly(ws)


@hard_timeout
def test_recv_timeout_accepts_the_largest_u64(server):
    # Effectively no deadline: it waits for the message and does not panic.
    ws = product_ws(server.url, "stock")
    subscribe = threading.Timer(0.3, ws.subscribe, args=(SUBSCRIPTION,))
    try:
        ws.connect()
        messages = ws.messages()
        assert next(messages)["event"] == "authenticated"
        subscribe.start()
        assert messages.recv_timeout(2**64 - 1)["event"] == "subscribed"
        with pytest.raises(OverflowError):
            messages.recv_timeout(2**64)
    finally:
        subscribe.cancel()
        subscribe.join()
        disconnect_quietly(ws)


@hard_timeout
def test_recv_timeout_still_times_out(server):
    ws = product_ws(server.url, "stock")
    try:
        ws.connect()
        messages = ws.messages()
        assert next(messages)["event"] == "authenticated"
        started = time.monotonic()
        assert messages.recv_timeout(250) is None
        assert 0.2 < time.monotonic() - started < 2
        assert messages.recv_timeout(0) is None
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_cancelled_anext_takes_no_message(server):
    # The message arrives within the 100 ms the cancelled wait used to linger.
    ws = product_ws(server.url, "stock")
    try:
        await ws.connect_async()
        messages = ws.messages()
        assert (await asyncio.wait_for(messages.__anext__(), 5))["event"] == "authenticated"
        for round_ in range(20):
            pending = asyncio.ensure_future(messages.__anext__())
            await asyncio.sleep(0.02)
            pending.cancel()
            await ws.subscribe_async({"channel": "trades", "symbol": str(round_)})
            for event in ("subscribed", "data"):
                msg = await asyncio.wait_for(messages.__anext__(), 5)
                assert msg["event"] == event, f"round {round_}: expected {event}, got {msg}"
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_cancelled_anext_does_not_delay_the_next_one(server):
    # A cancelled wait lingers up to 100 ms, waiting on the queue without
    # taking from it. Woken for a message, it must pass the wake-up on, or
    # the wait that replaced it only sees the message at its own next
    # wake-up, close to 100 ms later. Four are cancelled because the server
    # answers a subscribe with two frames, so two waits are woken. The median
    # leaves room for a loaded machine.
    ws = product_ws(server.url, "stock")
    delays = []
    try:
        await ws.connect_async()
        messages = ws.messages()
        assert (await asyncio.wait_for(messages.__anext__(), 5))["event"] == "authenticated"
        for round_ in range(20):
            cancelled = [asyncio.ensure_future(messages.__anext__()) for _ in range(4)]
            await asyncio.sleep(0.02)
            for pending in cancelled:
                pending.cancel()
            replacement = asyncio.ensure_future(messages.__anext__())
            started = time.monotonic()
            await ws.subscribe_async({"channel": "trades", "symbol": str(round_)})
            assert (await asyncio.wait_for(replacement, 5))["event"] == "subscribed"
            delays.append(time.monotonic() - started)
            assert (await asyncio.wait_for(messages.__anext__(), 5))["event"] == "data"
    finally:
        disconnect_quietly(ws)
    assert statistics.median(delays) < 0.05, f"delivery waited for a wake-up: {sorted(delays)}"


# Leaves an `__anext__` behind when `asyncio.run` closes the loop, then ends
# the connection, which completes the abandoned wait.
_LOOP_CLOSED_SCRIPT = """
import asyncio, sys, time
from tests.ws_loopback import LoopbackServer, disconnect_quietly, product_ws

async def main(ws):
    await ws.connect_async()
    messages = ws.messages()
    assert (await messages.__anext__())["event"] == "authenticated"
    pending = asyncio.ensure_future(messages.__anext__())
    await asyncio.sleep(0.2)
    if sys.argv[1] == "cancelled":
        pending.cancel()

with LoopbackServer() as srv:
    ws = product_ws(srv.url, "stock")
    asyncio.run(main(ws))
    time.sleep(0.3)
    disconnect_quietly(ws)
    time.sleep(0.5)
print("finished")
"""


@hard_timeout
@pytest.mark.parametrize("how", ["cancelled", "pending"])
def test_anext_outliving_its_event_loop_prints_nothing(how):
    done = subprocess.run(
        [sys.executable, "-c", _LOOP_CLOSED_SCRIPT, how],
        cwd=PY_DIR,
        capture_output=True,
        text=True,
        timeout=20,
    )
    assert done.returncode == 0, done.stderr
    assert done.stdout.strip() == "finished"
    assert "Traceback" not in done.stderr, done.stderr
    assert "Event loop is closed" not in done.stderr, done.stderr


# --- #267: `__anext__` resolves its awaitable at once while messages are
# queued, lets the event loop run once in every N such deliveries, and a wait
# left behind by a closed event loop ends on its own.

YIELD_EVERY_ENV = "FUGLE_MARKETDATA_AITER_YIELD_EVERY"
BACKLOG_READS = 640


@pytest.fixture
def flood_server():
    with LoopbackServer(flood=True) as srv:
        yield srv


async def _backlogged(ws):
    """Subscribe to the flooding server and return once the queue is full."""
    await ws.connect_async()
    await ws.subscribe_async(SUBSCRIPTION)
    deadline = time.monotonic() + 10
    while ws.messages_dropped_total() == 0:
        assert time.monotonic() < deadline, "the flood never filled the queue"
        await asyncio.sleep(0.05)


@hard_timeout
@pytest.mark.parametrize(
    "yield_every, fewest, most",
    [
        # One delivery in 32 goes through the loop, which takes two turns of
        # it: one runs the delivery, the next resumes the reader. The other
        # task runs in both, give or take the ones at either end.
        ("32", 2 * (BACKLOG_READS // 32) - 4, 2 * (BACKLOG_READS // 32) + 4),
        # Every delivery goes through the loop.
        ("1", 2 * BACKLOG_READS - 4, 2 * BACKLOG_READS + 4),
        # None does: the other task never runs while the backlog lasts.
        ("0", 0, 0),
    ],
)
async def test_backlogged_async_for_lets_other_tasks_run(flood_server, monkeypatch, yield_every, fewest, most):
    monkeypatch.setenv(YIELD_EVERY_ENV, yield_every)
    ws = product_ws(flood_server.url, "stock")
    turns = 0

    async def other_task():
        nonlocal turns
        while True:
            turns += 1
            await asyncio.sleep(0)

    try:
        await _backlogged(ws)
        other = asyncio.ensure_future(other_task())
        read = 0
        async for _ in ws.messages():
            read += 1
            if read == BACKLOG_READS:
                break
        seen = turns
        other.cancel()
    finally:
        disconnect_quietly(ws)
    assert fewest <= seen <= most, f"the other task ran {seen} times during {BACKLOG_READS} reads"


@hard_timeout
async def test_backlogged_anext_is_already_resolved_and_cannot_be_cancelled(flood_server, monkeypatch):
    # The behaviour with the fast path on (N > 1), as it is now: the message
    # is taken when `__anext__` is called, so `cancel()` fails and the message
    # stays in the awaitable. Code that ignores what `cancel()` returned and
    # drops the awaitable drops that message.
    monkeypatch.setenv(YIELD_EVERY_ENV, "32")
    ws = product_ws(flood_server.url, "stock")
    try:
        await _backlogged(ws)
        messages = ws.messages()
        first = messages.__anext__()
        assert first.done()
        assert first.cancel() is False
        assert first.result()["event"] == "authenticated"
        # The next read carries on after the one the awaitable holds.
        assert (await messages.__anext__())["event"] == "subscribed"
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_backlogged_anext_through_the_loop_can_be_cancelled(flood_server, monkeypatch):
    # With every delivery going through the loop (N = 1) nothing is taken
    # until the loop delivers, so cancelling works as it does with no backlog.
    monkeypatch.setenv(YIELD_EVERY_ENV, "1")
    ws = product_ws(flood_server.url, "stock")
    try:
        await _backlogged(ws)
        messages = ws.messages()
        first = messages.__anext__()
        assert not first.done()
        assert first.cancel() is True
        await asyncio.sleep(0)  # the delivery runs and finds it cancelled
        assert (await messages.__anext__())["event"] == "authenticated"
    finally:
        disconnect_quietly(ws)


@hard_timeout
async def test_backlogged_wait_for_loses_no_message(flood_server, monkeypatch):
    # `wait_for` wraps each step; none of the queued messages may go missing
    # or change places: authenticated, subscribed, then data only.
    monkeypatch.delenv(YIELD_EVERY_ENV, raising=False)  # the default
    ws = product_ws(flood_server.url, "stock")
    try:
        await _backlogged(ws)
        messages = ws.messages()
        events = [(await asyncio.wait_for(messages.__anext__(), 5))["event"] for _ in range(200)]
    finally:
        disconnect_quietly(ws)
    assert events == ["authenticated", "subscribed"] + ["data"] * 198


@hard_timeout
def test_waits_left_by_closed_event_loops_end(server):
    # Each `asyncio.run` leaves an `__anext__` that nobody cancelled. Its
    # wait on the blocking pool must end once the loop is closed, not stay
    # until the connection does.
    ws = product_ws(server.url, "stock")
    try:
        ws.connect()
        messages = ws.messages()
        assert next(messages)["event"] == "authenticated"
        pending_waits = type(messages)._pending_waits
        # The count covers the whole process: let what earlier tests left end.
        before = pending_waits()
        settled = time.monotonic()
        while time.monotonic() - settled < 1.5:
            time.sleep(0.1)
            if pending_waits() != before:
                before, settled = pending_waits(), time.monotonic()

        async def leave_one_pending():
            left = messages.__anext__()
            await asyncio.sleep(0.05)
            assert not left.done()
            assert pending_waits() > before

        for _ in range(5):
            asyncio.run(leave_one_pending())
        deadline = time.monotonic() + 5
        while pending_waits() > before and time.monotonic() < deadline:
            time.sleep(0.1)
        assert pending_waits() == before, "waits outlived their event loops"
        # The connection and the iterator still work.
        ws.subscribe(SUBSCRIPTION)
        assert messages.recv_timeout(5000)["event"] == "subscribed"
    finally:
        disconnect_quietly(ws)
