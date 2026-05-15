#!/usr/bin/env python3
"""WebSocket 即時串流測試 - 查看所有 server 回應"""

import os
import time
from fugle_marketdata import WebSocketClient

# 從環境變數取得 API key
api_key = os.environ.get("FUGLE_API_KEY")
if not api_key:
    print("請設定 FUGLE_API_KEY 環境變數")
    print("  export FUGLE_API_KEY='your-api-key'")
    exit(1)

# 建立 WebSocket client
ws = WebSocketClient(api_key)
stock = ws.stock

# 連線
print("連線中...")
stock.connect()
print(f"✓ 已連線 (is_connected: {stock.is_connected()})")

# 訂閱
print("\n訂閱 2330 trades...")
stock.subscribe("trades", "2330")
print("訂閱指令已送出")

# 取得 iterator 並接收訊息
print("\n等待 server 回應...\n")
iterator = stock.messages(timeout_ms=1000)

messages = []
start = time.time()
while time.time() - start < 5:
    try:
        msg = next(iterator)
        if msg:
            messages.append(msg)
            print(f"[收到] {msg}")
    except StopIteration:
        pass  # timeout

# 顯示結果
print(f"\n=== 總共收到 {len(messages)} 則訊息 ===")
for i, msg in enumerate(messages):
    print(f"\n訊息 {i+1}:")
    print(f"  event: {msg.get('event')}")
    print(f"  id: {msg.get('id')}")
    print(f"  channel: {msg.get('channel')}")
    print(f"  symbol: {msg.get('symbol')}")
    if msg.get('data'):
        print(f"  data: {msg.get('data')}")

print(f"\n訂閱清單: {stock.subscriptions()}")

# 斷線
print("\n斷線中...")
stock.disconnect()
print("完成!")
