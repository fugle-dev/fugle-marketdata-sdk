"""
Unit tests for v0.3.0 config classes and constructor patterns.

Tests:
- HealthCheckConfig construction and validation
- ReconnectConfig construction and validation
- RestClient kwargs constructor and auth validation
- WebSocketClient kwargs constructor with config params
"""
import pytest
from fugle_marketdata import (
    RestClient,
    WebSocketClient,
    ReconnectConfig,
    HealthCheckConfig,
)


class TestHealthCheckConfig:
    """Tests for HealthCheckConfig class."""

    def test_default_construction(self):
        """Default construction uses sensible defaults."""
        config = HealthCheckConfig()
        assert config.enabled == False  # Aligned with official SDKs
        assert config.interval_ms == 30000
        assert config.max_missed_pongs == 2

    def test_custom_values(self):
        """Can specify custom values via kwargs."""
        config = HealthCheckConfig(
            enabled=True,
            interval_ms=15000,
            max_missed_pongs=3
        )
        assert config.enabled == True
        assert config.interval_ms == 15000
        assert config.max_missed_pongs == 3

    def test_partial_kwargs(self):
        """Can override only some values."""
        config = HealthCheckConfig(enabled=True)
        assert config.enabled == True
        assert config.interval_ms == 30000  # Default

    def test_validation_interval_too_small(self):
        """interval_ms must be >= 5000."""
        with pytest.raises(ValueError) as exc_info:
            HealthCheckConfig(interval_ms=1000)
        assert "5000" in str(exc_info.value)  # Should mention minimum

    def test_validation_max_missed_pongs_zero(self):
        """max_missed_pongs must be >= 1."""
        with pytest.raises(ValueError) as exc_info:
            HealthCheckConfig(max_missed_pongs=0)
        assert "max_missed_pongs" in str(exc_info.value).lower() or "1" in str(exc_info.value)

    def test_fields_are_readable(self):
        """All fields can be read after construction."""
        config = HealthCheckConfig(enabled=True, interval_ms=10000, max_missed_pongs=5)
        # These should not raise AttributeError
        _ = config.enabled
        _ = config.interval_ms
        _ = config.max_missed_pongs


class TestReconnectConfig:
    """Tests for ReconnectConfig class (updated field names)."""

    def test_default_construction(self):
        """Default construction uses core-aligned defaults."""
        config = ReconnectConfig()
        assert config.enabled == True
        assert config.max_attempts == 5  # Was max_retries
        assert config.initial_delay_ms == 1000  # Was base_delay_ms
        assert config.max_delay_ms == 60000

    def test_custom_values(self):
        """Can specify custom values via kwargs."""
        config = ReconnectConfig(
            enabled=False,
            max_attempts=10,
            initial_delay_ms=2000,
            max_delay_ms=120000
        )
        assert config.enabled == False
        assert config.max_attempts == 10
        assert config.initial_delay_ms == 2000
        assert config.max_delay_ms == 120000

    def test_validation_max_attempts_zero(self):
        """max_attempts must be >= 1."""
        with pytest.raises(ValueError) as exc_info:
            ReconnectConfig(max_attempts=0)
        assert "max_attempts" in str(exc_info.value).lower() or "1" in str(exc_info.value)

    def test_validation_initial_delay_too_small(self):
        """initial_delay_ms must be >= 100."""
        with pytest.raises(ValueError) as exc_info:
            ReconnectConfig(initial_delay_ms=50)
        assert "100" in str(exc_info.value)  # Should mention minimum

    def test_validation_max_delay_less_than_initial(self):
        """max_delay_ms must be >= initial_delay_ms."""
        with pytest.raises(ValueError) as exc_info:
            ReconnectConfig(initial_delay_ms=10000, max_delay_ms=5000)
        assert "max_delay" in str(exc_info.value).lower() or "initial" in str(exc_info.value).lower()

    def test_static_default_config(self):
        """ReconnectConfig.default_config() creates enabled config."""
        config = ReconnectConfig.default_config()
        assert config.enabled == True
        assert config.max_attempts == 5

    def test_static_disabled(self):
        """ReconnectConfig.disabled() creates disabled config."""
        config = ReconnectConfig.disabled()
        assert config.enabled == False


class TestRestClientKwargsConstructor:
    """Tests for RestClient kwargs-based constructor."""

    def test_api_key_auth(self):
        """Can create client with api_key kwarg."""
        client = RestClient(api_key="test-key")
        assert client is not None

    def test_bearer_token_auth(self):
        """Can create client with bearer_token kwarg."""
        client = RestClient(bearer_token="test-token")
        assert client is not None

    def test_sdk_token_auth(self):
        """Can create client with sdk_token kwarg."""
        client = RestClient(sdk_token="test-sdk-token")
        assert client is not None

    def test_with_base_url(self):
        """Can specify custom base_url."""
        client = RestClient(api_key="key", base_url="https://custom.api")
        assert client is not None

    def test_no_auth_raises_error(self):
        """Must provide at least one auth method."""
        with pytest.raises(ValueError) as exc_info:
            RestClient()
        assert "exactly one" in str(exc_info.value).lower()

    def test_multiple_auth_raises_error(self):
        """Cannot provide multiple auth methods."""
        with pytest.raises(ValueError) as exc_info:
            RestClient(api_key="key", bearer_token="token")
        assert "exactly one" in str(exc_info.value).lower()

    def test_all_three_auth_raises_error(self):
        """Cannot provide all three auth methods."""
        with pytest.raises(ValueError) as exc_info:
            RestClient(api_key="k", bearer_token="t", sdk_token="s")
        assert "exactly one" in str(exc_info.value).lower()

    def test_static_methods_still_work(self):
        """Static methods remain for backwards compatibility."""
        client1 = RestClient.with_bearer_token("token")
        client2 = RestClient.with_sdk_token("sdk-token")
        assert client1 is not None
        assert client2 is not None


class TestWebSocketClientKwargsConstructor:
    """Tests for WebSocketClient kwargs-based constructor."""

    def test_api_key_auth(self):
        """Can create client with api_key kwarg."""
        ws = WebSocketClient(api_key="test-key")
        assert ws is not None

    def test_bearer_token_auth(self):
        """Can create client with bearer_token kwarg."""
        ws = WebSocketClient(bearer_token="test-token")
        assert ws is not None

    def test_sdk_token_auth(self):
        """Can create client with sdk_token kwarg."""
        ws = WebSocketClient(sdk_token="test-sdk-token")
        assert ws is not None

    def test_with_reconnect_config(self):
        """Can pass ReconnectConfig."""
        config = ReconnectConfig(max_attempts=10)
        ws = WebSocketClient(api_key="key", reconnect=config)
        assert ws is not None

    def test_with_health_check_config(self):
        """Can pass HealthCheckConfig."""
        config = HealthCheckConfig(enabled=True, interval_ms=15000)
        ws = WebSocketClient(api_key="key", health_check=config)
        assert ws is not None

    def test_with_both_configs(self):
        """Can pass both config objects."""
        rc = ReconnectConfig(max_attempts=10)
        hc = HealthCheckConfig(enabled=True)
        ws = WebSocketClient(api_key="key", reconnect=rc, health_check=hc)
        assert ws is not None

    def test_with_base_url(self):
        """Can specify custom base_url."""
        ws = WebSocketClient(api_key="key", base_url="wss://custom.ws")
        assert ws is not None

    def test_no_auth_raises_error(self):
        """Must provide at least one auth method."""
        with pytest.raises(ValueError) as exc_info:
            WebSocketClient()
        assert "exactly one" in str(exc_info.value).lower()

    def test_multiple_auth_raises_error(self):
        """Cannot provide multiple auth methods."""
        with pytest.raises(ValueError) as exc_info:
            WebSocketClient(api_key="key", bearer_token="token")
        assert "exactly one" in str(exc_info.value).lower()

    def test_has_stock_property(self):
        """ws.stock property still works."""
        ws = WebSocketClient(api_key="key")
        assert ws.stock is not None

    def test_has_futopt_property(self):
        """ws.futopt property still works."""
        ws = WebSocketClient(api_key="key")
        assert ws.futopt is not None
