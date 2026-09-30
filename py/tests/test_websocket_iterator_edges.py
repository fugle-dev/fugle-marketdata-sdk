"""Run-time edges of ``MessageIterator`` against a loopback server (#260).

``recv_timeout`` must let signal handlers run while it waits and accept any
``u64`` of milliseconds. A cancelled ``__anext__`` must not take a message:
its blocking wait used to outlive the cancellation by up to 100 ms and drop
whatever arrived meanwhile. An ``__anext__`` left pending when its event loop
closes must not print a traceback once the connection ends.
"""
import asyncio
import json
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
# queued, lets the event loop run once in every 32 such deliveries, and a wait
# left behind by a closed event loop ends on its own.

# One `__anext__` delivery in this many goes through the event loop
# (`YIELD_EVERY` in py/src/iterator.rs).
YIELD_EVERY = 32
BACKLOG_READS = 640
BURST = 100


@pytest.fixture
def flood_server():
    with LoopbackServer(flood=True) as srv:
        yield srv


@pytest.fixture
def burst_server():
    with LoopbackServer(burst_on_close=BURST) as srv:
        yield srv


async def _backlogged(ws):
    """Subscribe to the flooding server and return once the queue is full."""
    await ws.connect_async()
    await ws.subscribe_async(SUBSCRIPTION)
    deadline = time.monotonic() + 10
    while ws.messages_dropped_total() == 0:
        assert time.monotonic() < deadline, "the flood never filled the queue"
        await asyncio.sleep(0.05)


async def _closed_backlog(server, raw=False):
    """An iterator over a closed connection with a known backlog.

    The server answers the Close with `BURST` numbered frames, so after
    `disconnect()` the queue holds `authenticated` and those, and no more
    will come: what each read gets does not depend on timing.
    """
    ws = product_ws(server.url, "stock")
    await ws.connect_async()
    messages = ws.messages(raw=raw)
    ws.disconnect()
    return messages


def _label(msg):
    return msg["data"]["i"] if msg["event"] == "data" else msg["event"]


EVERYTHING = ["authenticated"] + list(range(BURST))


@hard_timeout
async def test_backlogged_async_for_lets_other_tasks_run(flood_server):
    # One delivery in 32 goes through the loop, which takes two turns of it:
    # one runs the delivery, the next resumes the reader. The other task
    # runs in both, give or take the ones at either end. Every delivery
    # through the loop would give it 1280 turns, none through it 0.
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
    expected = 2 * (BACKLOG_READS // YIELD_EVERY)
    assert expected - 4 <= seen <= expected + 4, f"the other task ran {seen} times during {BACKLOG_READS} reads"


@hard_timeout
async def test_async_for_reads_the_backlog_of_a_closed_connection_then_stops(burst_server):
    # Direct deliveries and the ones through the loop keep the order, and the
    # iteration ends only once the backlog is read.
    messages = await _closed_backlog(burst_server)
    assert [_label(msg) async for msg in messages] == EVERYTHING
    with pytest.raises(StopAsyncIteration):
        await messages.__anext__()


@hard_timeout
async def test_backlogged_raw_iterator_yields_the_frames_in_order(burst_server):
    messages = await _closed_backlog(burst_server, raw=True)
    first = messages.__anext__()
    assert first.done() and isinstance(first.result(), str)
    frames = [first.result()] + [frame async for frame in messages]
    assert all(isinstance(frame, str) for frame in frames)
    assert [_label(json.loads(frame)) for frame in frames] == EVERYTHING


@hard_timeout
async def test_backlogged_anext_is_already_resolved_and_cannot_be_cancelled(burst_server):
    # The message is taken when `__anext__` is called, so `cancel()` fails
    # and the message stays in the awaitable. Code that ignores what
    # `cancel()` returned and drops the awaitable drops that message.
    messages = await _closed_backlog(burst_server)
    first = asyncio.ensure_future(messages.__anext__())
    assert first.done()
    assert first.cancel() is False
    assert _label(first.result()) == "authenticated"
    # The next read carries on after the one the awaitable holds.
    assert _label(await messages.__anext__()) == 0


@hard_timeout
async def test_backlogged_anext_through_the_loop_can_be_cancelled(burst_server):
    # Every 32nd delivery in a row is left to the loop: that awaitable is
    # pending, and cancelling it takes no message, as with no backlog.
    messages = await _closed_backlog(burst_server)
    direct = [messages.__anext__() for _ in range(YIELD_EVERY - 1)]
    assert all(awaitable.done() for awaitable in direct)
    assert [_label(awaitable.result()) for awaitable in direct] == EVERYTHING[: YIELD_EVERY - 1]
    through_the_loop = messages.__anext__()
    assert not through_the_loop.done()
    assert through_the_loop.cancel() is True
    await asyncio.sleep(0)  # the delivery runs and finds it cancelled
    assert [_label(msg) async for msg in messages] == EVERYTHING[YIELD_EVERY - 1 :]


@hard_timeout
async def test_backlogged_wait_for_keeps_the_order(burst_server):
    # No step is cancelled or times out here: `wait_for` around each one
    # changes nothing about what is read.
    messages = await _closed_backlog(burst_server)
    labels = [_label(await asyncio.wait_for(messages.__anext__(), 5)) for _ in EVERYTHING]
    assert labels == EVERYTHING
    with pytest.raises(StopAsyncIteration):
        await asyncio.wait_for(messages.__anext__(), 5)


@hard_timeout
async def test_backlogged_wait_for_with_no_time_left_loses_no_message(burst_server):
    # `wait_for(..., 0)` returns an already resolved step and times out on
    # the ones left to the loop, which then take nothing.
    messages = await _closed_backlog(burst_server)
    labels = []
    timed_out = 0
    while len(labels) < len(EVERYTHING):
        try:
            labels.append(_label(await asyncio.wait_for(messages.__anext__(), 0)))
        except asyncio.TimeoutError:
            timed_out += 1
            assert timed_out < 50, labels
            await asyncio.sleep(0.005)
    assert labels == EVERYTHING
    # One in 32 went to the loop, and timed out.
    assert timed_out >= len(EVERYTHING) // YIELD_EVERY


@hard_timeout
async def test_backlogged_wait_for_with_time_left_loses_no_message(burst_server):
    # A timeout longer than a turn of the event loop never expires on a
    # queued message, the ones left to the loop included.
    messages = await _closed_backlog(burst_server)
    labels = [_label(await asyncio.wait_for(messages.__anext__(), 0.05)) for _ in EVERYTHING]
    assert labels == EVERYTHING


# `asyncio.wait_for` no longer takes a turn of the event loop for an
# awaitable that is already done.
WAIT_FOR_RETURNS_AT_ONCE = sys.version_info >= (3, 12)


@hard_timeout
async def test_cancelling_the_task_around_a_backlogged_wait_for_loses_no_message(burst_server):
    # The task is cancelled one turn of the loop after it started its step.
    messages = await _closed_backlog(burst_server)

    async def read():
        return await asyncio.wait_for(messages.__anext__(), 5)

    labels = []
    cancelled_steps = []
    steps = YIELD_EVERY + 8  # past a step left to the loop
    for step in range(1, steps + 1):
        task = asyncio.ensure_future(read())
        await asyncio.sleep(0)
        if WAIT_FOR_RETURNS_AT_ONCE and step != YIELD_EVERY:
            # The task finished in its first turn: nothing left to cancel.
            assert task.done() and task.cancel() is False
        else:
            assert not task.done() and task.cancel() is True
        try:
            labels.append(_label(await task))
        except asyncio.CancelledError:
            cancelled_steps.append(step)
    if WAIT_FOR_RETURNS_AT_ONCE:
        # Only the step left to the loop was still pending: the cancellation
        # reached its awaitable before the delivery ran, which then took no
        # message. The read after it got that message.
        assert cancelled_steps == [YIELD_EVERY]
        assert labels == EVERYTHING[: steps - 1]
    else:
        # This records what the standard library does, not the SDK: up to
        # 3.11 `wait_for` spends a turn of the loop even on an awaitable that
        # is already done, and (from 3.8.6) when cancelled in that turn it
        # returns the result instead of raising. The step left to the loop
        # is delivered within that turn, so it goes the same way. No message
        # is lost, and every cancellation is swallowed.
        assert cancelled_steps == []
        assert labels == EVERYTHING[:steps]


@hard_timeout
async def test_backlogged_wait_for_shorter_than_a_loop_turn_as_it_is_now(burst_server):
    # A known gap, recorded as it is (#267): a delivery left to the loop
    # resolves the awaitable one turn before the waiting task resumes. From
    # 3.12 a `wait_for` timeout that expires within that turn cancels the
    # task with the message already in the awaitable, and the message is
    # lost: the 32nd, 64th and 96th deliveries here. Up to 3.11 `wait_for`
    # looks at the awaitable first and returns the message.
    messages = await _closed_backlog(burst_server)
    labels = []
    for _ in range(3 * len(EVERYTHING)):
        try:
            labels.append(_label(await asyncio.wait_for(messages.__anext__(), 1e-6)))
        except asyncio.TimeoutError:
            await asyncio.sleep(0.005)
        if labels and labels[-1] == EVERYTHING[-1]:
            break
    lost = [EVERYTHING[n - 1] for n in range(YIELD_EVERY, len(EVERYTHING) + 1, YIELD_EVERY)]
    if WAIT_FOR_RETURNS_AT_ONCE:
        assert labels == [label for label in EVERYTHING if label not in lost]
    else:
        assert labels == EVERYTHING


@hard_timeout
async def test_checking_done_before_cancelling_keeps_the_backlogged_message(burst_server):
    # What the docs recommend for `asyncio.wait`: look at the step before
    # cancelling it, whatever else finished. With a backlog the step is
    # always done by then, a step left to the loop too: it is delivered in
    # the turn `wait` takes. Cancelling a pending step, which takes no
    # message, is test_cancelled_anext_takes_no_message and
    # test_backlogged_anext_through_the_loop_can_be_cancelled.
    messages = await _closed_backlog(burst_server)
    labels = []
    for _ in range(YIELD_EVERY + 8):  # past a step left to the loop
        step = asyncio.ensure_future(messages.__anext__())
        stop = asyncio.ensure_future(asyncio.sleep(0))
        await asyncio.wait({step, stop}, return_when=asyncio.FIRST_COMPLETED)
        assert step.done()
        labels.append(_label(step.result()))
        await stop
    rest = [_label(msg) async for msg in messages]
    assert labels + rest == EVERYTHING


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
        while time.monotonic() - settled < 2.5:
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
