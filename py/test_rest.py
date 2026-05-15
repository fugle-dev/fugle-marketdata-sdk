"""
Test script for fugle_marketdata REST client

This script tests:
1. Module import
2. RestClient creation with different auth methods
3. Client chain access (client.stock.intraday, client.futopt.intraday)
4. Error handling with error_code verification
"""

import os
import fugle_marketdata
from fugle_marketdata import RestClient, MarketDataError


def test_import():
    """Test module can be imported"""
    print("Testing import...")
    assert hasattr(fugle_marketdata, 'RestClient')
    assert hasattr(fugle_marketdata, 'MarketDataError')
    assert hasattr(fugle_marketdata, 'StockClient')
    assert hasattr(fugle_marketdata, 'StockIntradayClient')
    assert hasattr(fugle_marketdata, 'FutOptClient')
    assert hasattr(fugle_marketdata, 'FutOptIntradayClient')
    print("  Import OK")


def test_client_creation():
    """Test RestClient creation with different auth methods"""
    print("Testing client creation...")

    # API key authentication
    client1 = RestClient("test-api-key")
    print(f"  API key client: {client1}")

    # Bearer token authentication
    client2 = RestClient.with_bearer_token("test-token")
    print(f"  Bearer token client: {client2}")

    # SDK token authentication
    client3 = RestClient.with_sdk_token("test-sdk-token")
    print(f"  SDK token client: {client3}")

    print("  Client creation OK")


def test_client_chain():
    """Test client chain access pattern"""
    print("Testing client chain...")

    # 從環境變數取得 API key，若無則用測試用 key
    api_key = os.environ.get("FUGLE_API_KEY", "test-api-key")
    client = RestClient(api_key)

    # Stock chain
    stock = client.stock
    assert stock is not None
    print(f"  client.stock: {stock}")

    stock_intraday = client.stock.intraday
    assert stock_intraday is not None
    print(f"  client.stock.intraday: {stock_intraday}")

    # FutOpt chain
    futopt = client.futopt
    assert futopt is not None
    print(f"  client.futopt: {futopt}")

    futopt_intraday = client.futopt.intraday
    assert futopt_intraday is not None
    print(f"  client.futopt.intraday: {futopt_intraday}")

    print("  Client chain OK")


def test_error_handling():
    """Test error handling with error_code"""
    print("Testing error handling...")

    # MarketDataError should be an exception type
    assert issubclass(MarketDataError, Exception)
    print(f"  MarketDataError is Exception subclass: True")

    # Test that MarketDataError can be raised and caught
    try:
        raise MarketDataError("Test error", 1001)
    except MarketDataError as e:
        assert str(e.args[0]) == "Test error"
        assert e.args[1] == 1001  # error_code in args[1]
        print(f"  Error message: {e.args[0]}")
        print(f"  Error code: {e.args[1]}")

    print("  Error handling OK")


def test_api_call_with_invalid_key():
    """Test API call with invalid key returns proper error"""
    print("Testing API call with invalid key...")

    client = RestClient("invalid-key-that-will-fail")

    try:
        # This will fail because the API key is invalid
        # We expect an authentication or API error
        quote = client.stock.intraday.quote("2330")
        print(f"  Warning: API call succeeded unexpectedly: {quote}")
    except MarketDataError as e:
        message = e.args[0]
        error_code = e.args[1]
        print(f"  Expected error caught!")
        print(f"  Error message: {message}")
        print(f"  Error code: {error_code}")
        # Error codes:
        # 2002 = AuthError
        # 2003 = ApiError
        # 2001 = ConnectionError
        assert error_code in [2001, 2002, 2003], f"Unexpected error code: {error_code}"
    except Exception as e:
        print(f"  Unexpected exception type: {type(e).__name__}: {e}")
        raise

    print("  API call error handling OK")


def run_all_tests():
    """Run all tests"""
    print("=" * 60)
    print("fugle_marketdata REST Client Tests")
    print("=" * 60)

    test_import()
    test_client_creation()
    test_client_chain()
    test_error_handling()
    test_api_call_with_invalid_key()

    print("=" * 60)
    print("All tests passed!")
    print("=" * 60)


if __name__ == "__main__":
    run_all_tests()
