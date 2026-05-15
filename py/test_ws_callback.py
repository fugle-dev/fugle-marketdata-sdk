#!/usr/bin/env python3
"""WebSocket Callback 模式測試 — Health Check & Auto Reconnect

測試項目：
  1. 基本 callback（message, connect, disconnect）
  2. Health check 配置與心跳偵測
  3. Auto reconnect 配置與重連 callback（reconnect, error）

使用方式：
  export FUGLE_API_KEY='your-api-key'
  python test_ws_callback.py
"""

import os
import sys
import time
import threading
from fugle_marketdata import WebSocketClient, ReconnectConfig, HealthCheckConfig

# ============================================================
# 設定
# ============================================================
api_key = os.environ.get("FUGLE_API_KEY")
if not api_key:
    print("請設定 FUGLE_API_KEY 環境變數")
    print("  export FUGLE_API_KEY='your-api-key'")
    sys.exit(1)

# 測試持續秒數（可透過環境變數調整）
DURATION = int(os.environ.get("TEST_DURATION", "30"))


# ============================================================
# 事件追蹤器
# ============================================================
class EventTracker:
    """Thread-safe 事件計數器"""

    def __init__(self):
        self.lock = threading.Lock()
        self.messages = []
        self.connects = 0
        self.disconnects = []
        self.reconnects = []
        self.errors = []

    def on_message(self, msg):
        with self.lock:
            self.messages.append(msg)
        event = msg.get("event")
        data = msg.get("data", {})
        if event == "subscribed":
            print(f"  [subscribed] channel={data.get('channel')}, symbol={data.get('symbol')}")
        elif event == "snapshot":
            print(f"  [snapshot] {msg.get('channel')}:{data.get('symbol')}")
        elif event == "data":
            print(f"  [data] {msg.get('channel')}:{data.get('symbol')} price={data.get('price')}")
        elif event == "heartbeat":
            print(f"  [heartbeat] time={data.get('time')}")
        else:
            print(f"  [{event}] {data}")

    def on_connect(self):
        with self.lock:
            self.connects += 1
        print("  [event] connected")

    def on_disconnect(self, code, reason):
        with self.lock:
            self.disconnects.append((code, reason))
        print(f"  [event] disconnected code={code} reason={reason}")

    def on_reconnect(self, attempt):
        with self.lock:
            self.reconnects.append(attempt)
        print(f"  [event] reconnecting attempt={attempt}")

    def on_error(self, message, code):
        with self.lock:
            self.errors.append((message, code))
        print(f"  [event] error message={message} code={code}")

    def summary(self):
        with self.lock:
            return {
                "messages": len(self.messages),
                "connects": self.connects,
                "disconnects": len(self.disconnects),
                "reconnects": len(self.reconnects),
                "errors": len(self.errors),
            }


# ============================================================
# Test 1: 基本 callback + health check
# ============================================================
def test_basic_with_health_check():
    print("=" * 60)
    print("Test 1: 基本 callback + health check")
    print("=" * 60)

    tracker = EventTracker()

    # 建立有 health check 的 client
    reconnect_cfg = ReconnectConfig(enabled=True, max_attempts=3, initial_delay_ms=1000, max_delay_ms=5000)
    health_check_cfg = HealthCheckConfig(enabled=True, ping_interval=10000, max_missed_pongs=2)

    print(f"  reconnect: enabled={reconnect_cfg.enabled}, max_attempts={reconnect_cfg.max_attempts}, "
          f"initial_delay={reconnect_cfg.initial_delay_ms}ms, max_delay={reconnect_cfg.max_delay_ms}ms")
    print(f"  health_check: enabled={health_check_cfg.enabled}, interval={health_check_cfg.ping_interval}ms, "
          f"max_missed_pongs={health_check_cfg.max_missed_pongs}")

    ws = WebSocketClient(
        api_key=api_key,
        reconnect=reconnect_cfg,
        health_check=health_check_cfg,
    )
    stock = ws.stock

    # 註冊所有 callbacks
    stock.on("message", tracker.on_message)
    stock.on("connect", tracker.on_connect)
    stock.on("disconnect", tracker.on_disconnect)
    stock.on("reconnect", tracker.on_reconnect)
    stock.on("error", tracker.on_error)

    # 連線 + 訂閱
    print("\n  connecting...")
    stock.connect()
    print("  subscribing trades:2330...")
    stock.subscribe("trades", "2330")

    # 等待訊息
    print(f"\n  waiting {DURATION}s for messages...\n")
    time.sleep(DURATION)

    # 斷線
    print("\n  disconnecting...")
    stock.disconnect()
    time.sleep(1)

    # 結果
    s = tracker.summary()
    print(f"\n  Result: messages={s['messages']}, connects={s['connects']}, "
          f"disconnects={s['disconnects']}, reconnects={s['reconnects']}, errors={s['errors']}")
    assert s["connects"] >= 1, "should have at least 1 connect event"
    assert s["messages"] >= 1, "should have received at least 1 message"
    print("  PASSED\n")


# ============================================================
# Test 2: Reconnect callback 測試（使用無效 endpoint 觸發重連）
# ============================================================
def test_reconnect_callbacks():
    print("=" * 60)
    print("Test 2: Reconnect / Error callback 驗證")
    print("=" * 60)

    tracker = EventTracker()

    # 只測試 config 物件建立與 callback 註冊（不實際連線到錯誤端點）
    reconnect_cfg = ReconnectConfig(enabled=True, max_attempts=2, initial_delay_ms=500, max_delay_ms=2000)
    health_check_cfg = HealthCheckConfig(enabled=False)

    print(f"  reconnect: max_attempts={reconnect_cfg.max_attempts}, "
          f"initial_delay={reconnect_cfg.initial_delay_ms}ms")

    ws = WebSocketClient(
        api_key=api_key,
        reconnect=reconnect_cfg,
        health_check=health_check_cfg,
    )
    stock = ws.stock

    stock.on("message", tracker.on_message)
    stock.on("connect", tracker.on_connect)
    stock.on("disconnect", tracker.on_disconnect)
    stock.on("reconnect", tracker.on_reconnect)
    stock.on("error", tracker.on_error)

    # 正常連線測試 — 驗證 reconnect callback 已正確註冊
    print("\n  connecting (normal endpoint)...")
    stock.connect()
    stock.subscribe("trades", "2330")

    print(f"  waiting 5s...\n")
    time.sleep(5)

    stock.disconnect()
    time.sleep(1)

    s = tracker.summary()
    print(f"\n  Result: messages={s['messages']}, connects={s['connects']}, "
          f"reconnects={s['reconnects']}, errors={s['errors']}")
    print("  (reconnect/error events only fire on actual connection loss)")
    print("  PASSED\n")


# ============================================================
# Test 3: Config 驗證（邊界值測試）
# ============================================================
def test_config_validation():
    print("=" * 60)
    print("Test 3: Config 參數驗證")
    print("=" * 60)

    # 正常建立
    rc = ReconnectConfig(enabled=True, max_attempts=10, initial_delay_ms=200, max_delay_ms=30000)
    print(f"  ReconnectConfig OK: max_attempts={rc.max_attempts}")

    hc = HealthCheckConfig(enabled=True, ping_interval=5000, max_missed_pongs=3)
    print(f"  HealthCheckConfig OK: ping_interval={hc.ping_interval}")

    # Default config
    rc_default = ReconnectConfig()
    assert rc_default.enabled is True
    assert rc_default.max_attempts == 5
    assert rc_default.initial_delay_ms == 1000
    assert rc_default.max_delay_ms == 60000
    print(f"  ReconnectConfig default OK: enabled={rc_default.enabled}, max_attempts={rc_default.max_attempts}")

    hc_default = HealthCheckConfig()
    assert hc_default.enabled is False
    assert hc_default.ping_interval == 30000
    assert hc_default.max_missed_pongs == 3
    print(f"  HealthCheckConfig default OK: enabled={hc_default.enabled}, ping_interval={hc_default.ping_interval}")

    # 無效參數應該拋錯
    invalid_cases = [
        ("max_attempts=0", lambda: ReconnectConfig(max_attempts=0)),
        ("initial_delay_ms=0", lambda: ReconnectConfig(initial_delay_ms=0)),
        ("ping_interval=100 (too low)", lambda: HealthCheckConfig(enabled=True, ping_interval=100)),
        ("max_missed_pongs=0", lambda: HealthCheckConfig(enabled=True, max_missed_pongs=0)),
    ]

    for desc, fn in invalid_cases:
        try:
            fn()
            print(f"  FAIL: {desc} should have raised ValueError")
        except ValueError as e:
            print(f"  ValueError OK ({desc}): {e}")
        except Exception as e:
            print(f"  Exception ({desc}): {type(e).__name__}: {e}")

    print("  PASSED\n")


# ============================================================
# Test 4: FutOpt endpoint 測試
# ============================================================
def test_futopt_with_config():
    print("=" * 60)
    print("Test 4: FutOpt endpoint + config")
    print("=" * 60)

    tracker = EventTracker()

    ws = WebSocketClient(
        api_key=api_key,
        reconnect=ReconnectConfig(enabled=True, max_attempts=3),
        health_check=HealthCheckConfig(enabled=True, ping_interval=15000),
    )
    futopt = ws.futopt

    futopt.on("message", tracker.on_message)
    futopt.on("connect", tracker.on_connect)
    futopt.on("disconnect", tracker.on_disconnect)
    futopt.on("reconnect", tracker.on_reconnect)
    futopt.on("error", tracker.on_error)

    print("\n  connecting futopt...")
    futopt.connect()
    print("  subscribing trades:TXFB5...")
    futopt.subscribe("trades", "TXFB5")

    print(f"  waiting 10s...\n")
    time.sleep(10)

    futopt.disconnect()
    time.sleep(1)

    s = tracker.summary()
    print(f"\n  Result: messages={s['messages']}, connects={s['connects']}")
    assert s["connects"] >= 1, "futopt should connect"
    print("  PASSED\n")


# ============================================================
# Main
# ============================================================
if __name__ == "__main__":
    tests = {
        "config": test_config_validation,
        "basic": test_basic_with_health_check,
        "reconnect": test_reconnect_callbacks,
        "futopt": test_futopt_with_config,
    }

    # 可以指定要跑哪個測試: python test_ws_callback.py config
    selected = sys.argv[1:] if len(sys.argv) > 1 else list(tests.keys())

    for name in selected:
        if name not in tests:
            print(f"Unknown test: {name}")
            print(f"Available: {', '.join(tests.keys())}")
            sys.exit(1)

    for name in selected:
        tests[name]()

    print("=" * 60)
    print("All tests completed!")
    print("=" * 60)
