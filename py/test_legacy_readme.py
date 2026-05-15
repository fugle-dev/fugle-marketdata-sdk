#!/usr/bin/env python3
"""Smoke test mirroring fugle-marketdata-python's README examples.

Each section corresponds to a code block in the legacy README so you can
eyeball-compare API surface and runtime behavior. The only intentional
difference is the import path:

    Legacy:  from fugle_marketdata import WebSocketClient, RestClient
    Ours:    from fugle_marketdata    import WebSocketClient, RestClient

Run individual sections, or all of them:

    export FUGLE_API_KEY='your-key'
    python test_legacy_readme.py rest
    python test_legacy_readme.py ws
    python test_legacy_readme.py errors
    python test_legacy_readme.py             # all sections
"""

import os
import sys
import time

from fugle_marketdata import (
    RestClient,
    WebSocketClient,
    FugleAPIError,
)


API_KEY = os.environ.get("FUGLE_API_KEY")
if not API_KEY:
    print("Set FUGLE_API_KEY environment variable first.")
    sys.exit(1)


# ---------------------------------------------------------------------------
# Section 1 — REST API (legacy README "REST API" block)
# ---------------------------------------------------------------------------
def section_rest():
    print("=" * 60)
    print("Section 1: REST API")
    print("=" * 60)

    client = RestClient(api_key=API_KEY)
    stock = client.stock  # Stock REST API client
    print(stock.intraday.quote(symbol="2330"))
    print()


# ---------------------------------------------------------------------------
# Section 2 — WebSocket API (legacy README "WebSocket API" block)
# ---------------------------------------------------------------------------
def section_ws():
    print("=" * 60)
    print("Section 2: WebSocket API")
    print("=" * 60)

    def handle_message(message):
        print(f"message: {message}")

    def handle_connect():
        print("connected")

    def handle_disconnect(code, message):
        print(f"disconnect: {code}, {message}")

    def handle_error(error, code):
        # NOTE: legacy README shows `handle_error(error)` but the new SDK
        # passes (message, code). Both fields surface here for parity.
        print(f"error: {error} (code={code})")

    client = WebSocketClient(api_key=API_KEY)
    stock = client.stock
    stock.on("connect", handle_connect)
    stock.on("message", handle_message)
    stock.on("disconnect", handle_disconnect)
    stock.on("error", handle_error)
    stock.connect()
    stock.subscribe({
        "channel": "trades",
        "symbol": "2330",
    })

    # Stream for ~10 seconds then disconnect (the README example never
    # disconnects; we add a bounded wait so the script can finish).
    time.sleep(10)
    stock.disconnect()
    print()


# ---------------------------------------------------------------------------
# Section 3 — Error handling
#
# Maps to two adjacent blocks in the legacy README ("Catching API Errors"
# and "Common Error Scenarios"). Both are exercised here as 3a / 3b.
# ---------------------------------------------------------------------------
def section_errors():
    print("=" * 60)
    print("Section 3: Error handling")
    print("=" * 60)

    client = RestClient(api_key=API_KEY)

    # 3a. Happy path: legitimate symbol should succeed.
    print("\n--- 3a. Successful quote (sanity check) ---")
    print(">>> client.stock.intraday.quote(symbol='2330')")
    try:
        data = client.stock.intraday.quote(symbol="2330")
        print(f"OK: got quote for 2330 ({len(str(data))} bytes)")
    except FugleAPIError as e:
        print(f"UNEXPECTED error on a valid symbol: {e}")

    # 3b. Error path: an invalid symbol intentionally triggers a 404 so we
    # can validate that FugleAPIError is raised and catchable. The 404 is
    # the *expected* outcome of this test, not a bug.
    print("\n--- 3b. Intentional 404 (testing error handling) ---")
    print(">>> client.stock.intraday.quote(symbol='INVALID_SYMBOL')")
    print("    (we expect a 404 here — this validates the FugleAPIError catch path)")
    try:
        client.stock.intraday.quote(symbol="INVALID_SYMBOL")
        print("UNEXPECTED: invalid symbol did not raise")
    except FugleAPIError as e:
        # NOTE: the legacy README also accesses e.message / e.url /
        # e.status_code / e.params / e.response_text. Our MarketDataError
        # alias does not expose those as attributes yet — only `str(e)`
        # and the exception type are validated here. Adding the legacy
        # attribute surface is tracked as a follow-up.
        print(f"  caught FugleAPIError (expected): {e}")
        print(f"  type: {type(e).__name__}")
    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
SECTIONS = {
    "rest": section_rest,
    "ws": section_ws,
    "errors": section_errors,
}


def main():
    selected = sys.argv[1:] or list(SECTIONS.keys())
    for name in selected:
        if name not in SECTIONS:
            print(f"Unknown section: {name}")
            print(f"Available: {', '.join(SECTIONS.keys())}")
            sys.exit(1)
    for name in selected:
        SECTIONS[name]()


if __name__ == "__main__":
    main()
