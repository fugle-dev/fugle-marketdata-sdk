"""Pytest configuration for fugle_marketdata tests."""
import os
import pytest
from fugle_marketdata import RestClient, WebSocketClient

# Get API key from environment for integration tests
API_KEY = os.environ.get("FUGLE_API_KEY", "test-api-key")


@pytest.fixture
def api_key():
    """Provide API key for tests."""
    return API_KEY


@pytest.fixture
def rest_client(api_key):
    """Create REST client for tests."""
    return RestClient(api_key)


@pytest.fixture
def ws_client(api_key):
    """Create WebSocket client for tests."""
    return WebSocketClient(api_key)


@pytest.fixture
def mock_api_key():
    """Provide mock API key for unit tests (no network)."""
    return "mock-api-key-for-unit-tests"


def pytest_collection_modifyitems(config, items):
    """Skip integration tests if no API key is set."""
    if not os.environ.get("FUGLE_API_KEY"):
        skip_integration = pytest.mark.skip(reason="FUGLE_API_KEY not set")
        for item in items:
            if "integration" in item.keywords:
                item.add_marker(skip_integration)
