#!/usr/bin/env python3
"""
Test script for WebSocket bindings.

Tests:
1. WebSocketClient creation and attribute access
2. Callback registration (on/off)
3. StockWebSocketClient methods
4. FutOptWebSocketClient methods
5. MessageIterator creation
6. Basic GC check

Note: Actual connection tests require a valid API key and network access.
"""

import gc
import sys


def test_import():
    """Test importing the module."""
    print("Test 1: Import module...")
    import fugle_marketdata
    print(f"  Module: {fugle_marketdata}")
    print(f"  WebSocketClient: {fugle_marketdata.WebSocketClient}")
    print(f"  StockWebSocketClient: {fugle_marketdata.StockWebSocketClient}")
    print(f"  FutOptWebSocketClient: {fugle_marketdata.FutOptWebSocketClient}")
    print(f"  MessageIterator: {fugle_marketdata.MessageIterator}")
    print("  OK\n")
    return fugle_marketdata


def test_client_creation(fugle_marketdata):
    """Test WebSocket client creation."""
    print("Test 2: Create WebSocketClient...")
    ws = fugle_marketdata.WebSocketClient("test-api-key")
    print(f"  Client: {ws}")

    # Access stock and futopt getters
    stock = ws.stock
    print(f"  Stock client: {stock}")

    futopt = ws.futopt
    print(f"  FutOpt client: {futopt}")
    print("  OK\n")
    return ws


def test_callback_registration(fugle_marketdata):
    """Test callback registration."""
    print("Test 3: Test callback registration...")
    ws = fugle_marketdata.WebSocketClient("test-api-key")

    # Register callbacks
    messages_received = []

    def on_message(msg):
        messages_received.append(msg)

    def on_connect():
        print("    Connected!")

    def on_disconnect(code, reason):
        print(f"    Disconnected: {code} - {reason}")

    def on_error(message, code):
        print(f"    Error: {code} - {message}")

    # Register on stock client
    stock = ws.stock
    stock.on("message", on_message)
    stock.on("connect", on_connect)
    stock.on("disconnect", on_disconnect)
    stock.on("error", on_error)
    print("  Callbacks registered")

    # Test off (unregister)
    stock.off("message")
    print("  Message callback unregistered")

    # Test invalid event type
    try:
        stock.on("invalid_event", on_message)
        print("  ERROR: Should have raised ValueError")
        return False
    except ValueError as e:
        print(f"  Invalid event correctly rejected: {e}")

    # Test non-callable
    try:
        stock.on("message", "not a function")
        print("  ERROR: Should have raised TypeError")
        return False
    except TypeError as e:
        print(f"  Non-callable correctly rejected: {e}")

    print("  OK\n")
    return True


def test_not_connected_error(fugle_marketdata):
    """Test error when not connected."""
    print("Test 4: Test not connected errors...")
    ws = fugle_marketdata.WebSocketClient("test-api-key")
    stock = ws.stock

    # is_connected should return False
    assert not stock.is_connected(), "Should not be connected"
    print("  is_connected() returns False")

    # subscribe should fail when not connected
    try:
        stock.subscribe("trades", "2330")
        print("  ERROR: Should have raised RuntimeError")
        return False
    except RuntimeError as e:
        print(f"  Subscribe correctly fails when not connected: {e}")

    # unsubscribe should fail when not connected
    try:
        stock.unsubscribe("test-id")
        print("  ERROR: Should have raised RuntimeError")
        return False
    except RuntimeError as e:
        print(f"  Unsubscribe correctly fails when not connected: {e}")

    # messages() should fail when not connected
    try:
        _ = stock.messages()
        print("  ERROR: Should have raised RuntimeError")
        return False
    except RuntimeError as e:
        print(f"  Messages correctly fails when not connected: {e}")

    print("  OK\n")
    return True


def test_futopt_client(fugle_marketdata):
    """Test FutOpt client."""
    print("Test 5: Test FutOpt client...")
    ws = fugle_marketdata.WebSocketClient("test-api-key")
    futopt = ws.futopt

    # Verify it's the correct type
    assert isinstance(futopt, fugle_marketdata.FutOptWebSocketClient)
    print(f"  FutOpt client type: {type(futopt)}")

    # Test callback registration
    futopt.on("message", lambda msg: None)
    futopt.off("message")
    print("  Callbacks work")

    # Test is_connected
    assert not futopt.is_connected()
    print("  is_connected() returns False")

    # Test invalid channel should fail during subscribe
    # (but we can't test this without connect)

    print("  OK\n")
    return True


def test_gc_check(fugle_marketdata):
    """Basic GC check - ensure no obvious memory leaks."""
    print("Test 6: Basic GC check...")

    # Create and destroy multiple clients
    for i in range(10):
        ws = fugle_marketdata.WebSocketClient("test-api-key")
        stock = ws.stock
        futopt = ws.futopt

        # Register callbacks
        stock.on("message", lambda msg: msg)
        stock.on("connect", lambda: None)
        futopt.on("error", lambda m, c: None)

        # Unregister
        stock.off("message")
        stock.off("connect")
        futopt.off("error")

        # Delete references
        del stock
        del futopt
        del ws

    # Force GC
    gc.collect()
    gc.collect()
    gc.collect()

    print(f"  GC stats: {gc.get_stats()}")
    print("  No crashes during GC - OK\n")
    return True


def test_subscriptions_list(fugle_marketdata):
    """Test subscriptions() returns empty list when not connected."""
    print("Test 7: Test subscriptions list...")
    ws = fugle_marketdata.WebSocketClient("test-api-key")
    stock = ws.stock
    futopt = ws.futopt

    # Should return empty list
    subs = stock.subscriptions()
    assert subs == [], f"Expected empty list, got {subs}"
    print(f"  Stock subscriptions: {subs}")

    subs = futopt.subscriptions()
    assert subs == [], f"Expected empty list, got {subs}"
    print(f"  FutOpt subscriptions: {subs}")

    print("  OK\n")
    return True


def main():
    """Run all tests."""
    print("=" * 60)
    print("WebSocket Bindings Test Suite")
    print("=" * 60 + "\n")

    try:
        fugle_marketdata = test_import()
        test_client_creation(fugle_marketdata)

        if not test_callback_registration(fugle_marketdata):
            return 1

        if not test_not_connected_error(fugle_marketdata):
            return 1

        if not test_futopt_client(fugle_marketdata):
            return 1

        if not test_gc_check(fugle_marketdata):
            return 1

        if not test_subscriptions_list(fugle_marketdata):
            return 1

        print("=" * 60)
        print("All tests passed!")
        print("=" * 60)
        return 0

    except Exception as e:
        print(f"\nERROR: {type(e).__name__}: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
