#!/usr/bin/env python3
"""Async REST API 範例"""

import os
import asyncio
from fugle_marketdata import RestClient

# 從環境變數取得 API key
api_key = os.environ.get("FUGLE_API_KEY")
if not api_key:
    print("請設定 FUGLE_API_KEY 環境變數")
    print("  export FUGLE_API_KEY='your-api-key'")
    exit(1)


async def test():
    client = RestClient(api_key)
    print(f"Stock client: {client.stock}")
    try:
        response = await client.stock.intraday.quote("2330")
        print(f"Intraday quote response: {response}")
    except Exception as e:
        print(f"Error: {e}")


asyncio.run(test())
