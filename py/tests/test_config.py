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
    MarketDataError,
    ConfigError,
)


def assert_credentials_rejected(exc_info):
    """Credential errors come from core as ConfigError (code 1004)."""
    assert_config_error(exc_info)
    assert "exactly one non-empty credential" in str(exc_info.value)


def assert_config_error(exc_info):
    """A configuration error is a ``ConfigError`` carrying the unified
    fields (#171): code 1004 and ``source_kind == "client"``, as the other
    four languages report it."""
    assert isinstance(exc_info.value, ConfigError)
    assert exc_info.value.code == 1004
    assert exc_info.value.source_kind == "client"
    assert exc_info.value.args == (exc_info.value.message, 1004)


def test_config_error_is_a_market_data_error():
    """``ConfigError`` hangs under ``MarketDataError`` like every other SDK
    exception, so ``except MarketDataError`` still catches it. It is not a
    ``ValueError``: that was the pre-3.0.0 behaviour of ``ReconnectConfig``
    and ``HealthCheckConfig`` (#171)."""
    assert issubclass(ConfigError, MarketDataError)
    assert not issubclass(ConfigError, ValueError)


class TestHealthCheckConfig:
    """Tests for HealthCheckConfig class."""

    # NOTE: this class does NOT mirror the official SDK's HealthCheckConfig
    # field-for-field. The official SDKs poll on a timer and count missed
    # pongs (`ping_interval`, `max_missed_pongs`); this SDK uses a single
    # timeout window enforced at the read site, so there is no interval to
    # set and no counter to clamp. See the mapping table in
    # `websocket::health_check`'s module docs.

    def test_default_construction(self):
        """Default construction uses sensible defaults."""
        config = HealthCheckConfig()
        # Enabled by default since 3.0 — a silent connection would otherwise
        # sit unnoticed until the OS times out the TCP socket.
        assert config.enabled is True
        # Server heartbeat is 30s; +5s absorbs network jitter.
        assert config.heartbeat_timeout_ms == 35000

    def test_custom_values(self):
        """Can specify custom values via kwargs."""
        config = HealthCheckConfig(enabled=True, heartbeat_timeout_ms=60000)
        assert config.enabled is True
        assert config.heartbeat_timeout_ms == 60000

    def test_partial_kwargs(self):
        """Can override only some values."""
        config = HealthCheckConfig(enabled=False)
        assert config.enabled is False
        assert config.heartbeat_timeout_ms == 35000  # Default

    def test_validation_timeout_too_small(self):
        """heartbeat_timeout_ms must be >= 5000; a ConfigError, code 1004."""
        with pytest.raises(ConfigError) as exc_info:
            HealthCheckConfig(heartbeat_timeout_ms=1000)
        assert_config_error(exc_info)
        assert "5000" in str(exc_info.value)  # Should mention minimum

    def test_validation_error_is_not_a_value_error(self):
        """``except ValueError`` no longer catches it (#171)."""
        with pytest.raises(MarketDataError) as exc_info:
            HealthCheckConfig(heartbeat_timeout_ms=1000)
        assert not isinstance(exc_info.value, ValueError)

    def test_validation_floor_is_accepted(self):
        """5000 is at the floor and must be accepted."""
        assert HealthCheckConfig(heartbeat_timeout_ms=5000).heartbeat_timeout_ms == 5000

    def test_validation_runs_even_when_disabled(self):
        """Bad input is rejected up front, not silently kept for later."""
        with pytest.raises(ConfigError) as exc_info:
            HealthCheckConfig(enabled=False, heartbeat_timeout_ms=100)
        assert_config_error(exc_info)

    def test_rejects_official_sdk_field_names(self):
        """`ping_interval` / `max_missed_pongs` have no counterpart here.

        Accepting them silently would be worse than rejecting: a caller
        porting config from the Node or Python SDK would believe they had
        tuned the health check when nothing had changed.
        """
        with pytest.raises(TypeError):
            HealthCheckConfig(ping_interval=15000)
        with pytest.raises(TypeError):
            HealthCheckConfig(max_missed_pongs=3)

    def test_fields_are_readable(self):
        """All fields can be read after construction."""
        config = HealthCheckConfig(enabled=True, heartbeat_timeout_ms=10000)
        assert config.enabled is True
        assert config.heartbeat_timeout_ms == 10000

    def test_probe_defaults(self):
        """Probe mode is opt-in; its defaults keep detection at 35s."""
        config = HealthCheckConfig()
        assert config.probe_enabled is False
        # Matches the server's 30s heartbeat: no ping while it is on time.
        assert config.idle_probe_after_ms == 30000
        assert config.probe_timeout_ms == 5000
        assert config.idle_probe_after_ms + config.probe_timeout_ms == config.heartbeat_timeout_ms

    def test_probe_values(self):
        config = HealthCheckConfig(probe_enabled=True, idle_probe_after_ms=5000, probe_timeout_ms=1000)
        assert config.probe_enabled is True
        assert config.idle_probe_after_ms == 5000
        assert config.probe_timeout_ms == 1000

    def test_probe_floors(self):
        with pytest.raises(ConfigError, match="5000") as exc_info:
            HealthCheckConfig(probe_enabled=True, idle_probe_after_ms=4999)
        assert_config_error(exc_info)
        with pytest.raises(ConfigError, match="1000") as exc_info:
            HealthCheckConfig(probe_enabled=True, probe_timeout_ms=999)
        assert_config_error(exc_info)

    def test_probe_floors_checked_with_probe_off(self):
        """A bad probe setting is rejected before it is switched on."""
        with pytest.raises(ConfigError) as exc_info:
            HealthCheckConfig(probe_timeout_ms=10)
        assert_config_error(exc_info)


class TestReconnectConfig:
    """Tests for ReconnectConfig class (updated field names)."""

    def test_default_construction(self):
        """Default construction uses core-aligned defaults."""
        config = ReconnectConfig()
        assert config.enabled == True
        assert config.max_attempts == 0  # unlimited (#149)
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

    def test_max_attempts_zero_is_unlimited(self):
        """max_attempts=0 means unlimited attempts, not a validation error."""
        config = ReconnectConfig(max_attempts=0)
        assert config.max_attempts == 0

    @pytest.mark.parametrize("initial_delay_ms", [1, 50])
    def test_validation_initial_delay_too_small(self, initial_delay_ms):
        """initial_delay_ms must be >= 100; a ConfigError, code 1004."""
        with pytest.raises(ConfigError) as exc_info:
            ReconnectConfig(initial_delay_ms=initial_delay_ms)
        assert_config_error(exc_info)
        assert "100" in str(exc_info.value)  # Should mention minimum

    def test_validation_max_delay_less_than_initial(self):
        """max_delay_ms must be >= initial_delay_ms; a ConfigError, code 1004."""
        with pytest.raises(ConfigError) as exc_info:
            ReconnectConfig(initial_delay_ms=10000, max_delay_ms=5000)
        assert_config_error(exc_info)
        assert "max_delay" in str(exc_info.value).lower() or "initial" in str(exc_info.value).lower()

    def test_validation_error_is_not_a_value_error(self):
        """``except ValueError`` no longer catches it (#171)."""
        with pytest.raises(MarketDataError) as exc_info:
            ReconnectConfig(initial_delay_ms=1)
        assert not isinstance(exc_info.value, ValueError)

    def test_static_default_config(self):
        """ReconnectConfig.default_config() creates enabled config."""
        config = ReconnectConfig.default_config()
        assert config.enabled == True
        assert config.max_attempts == 0

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
        with pytest.raises(MarketDataError) as exc_info:
            RestClient()
        assert_credentials_rejected(exc_info)

    def test_multiple_auth_raises_error(self):
        """Cannot provide multiple auth methods."""
        with pytest.raises(MarketDataError) as exc_info:
            RestClient(api_key="key", bearer_token="token")
        assert_credentials_rejected(exc_info)

    def test_all_three_auth_raises_error(self):
        """Cannot provide all three auth methods."""
        with pytest.raises(MarketDataError) as exc_info:
            RestClient(api_key="k", bearer_token="t", sdk_token="s")
        assert_credentials_rejected(exc_info)

    @pytest.mark.parametrize(
        "kwargs",
        [
            {"api_key": ""},
            {"bearer_token": "   "},
            {"sdk_token": "\t\n"},
            {"api_key": "", "bearer_token": ""},
        ],
    )
    def test_blank_auth_raises_error(self, kwargs):
        """Empty or whitespace-only credentials count as not provided."""
        with pytest.raises(MarketDataError) as exc_info:
            RestClient(**kwargs)
        assert_credentials_rejected(exc_info)

    def test_blank_auth_is_skipped_when_another_is_given(self):
        """A blank credential next to a real one is ignored."""
        assert RestClient(api_key="", bearer_token="token") is not None

    def test_static_methods_reject_blank_token(self):
        """with_bearer_token / with_sdk_token apply the same rule."""
        with pytest.raises(MarketDataError) as exc_info:
            RestClient.with_bearer_token("")
        assert_credentials_rejected(exc_info)
        with pytest.raises(MarketDataError) as exc_info:
            RestClient.with_sdk_token("  ")
        assert_credentials_rejected(exc_info)

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
        config = HealthCheckConfig(enabled=True, heartbeat_timeout_ms=15000)
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

    def test_with_auth_timeout_ms(self):
        """auth_timeout_ms bounds the auth handshake (#199); default 10 s."""
        ws = WebSocketClient(api_key="key", auth_timeout_ms=15000)
        assert ws is not None

    @pytest.mark.parametrize("auth_timeout_ms", [0, -1])
    def test_auth_timeout_ms_below_one_raises_config_error(self, auth_timeout_ms):
        """Core validates it: a ConfigError, code 1004, not a ValueError."""
        with pytest.raises(ConfigError) as exc_info:
            WebSocketClient(api_key="key", auth_timeout_ms=auth_timeout_ms)
        assert exc_info.value.code == 1004
        assert "auth_timeout_ms must be greater than 0" in str(exc_info.value)

    def test_no_auth_raises_error(self):
        """Must provide at least one auth method."""
        with pytest.raises(MarketDataError) as exc_info:
            WebSocketClient()
        assert_credentials_rejected(exc_info)

    def test_multiple_auth_raises_error(self):
        """Cannot provide multiple auth methods."""
        with pytest.raises(MarketDataError) as exc_info:
            WebSocketClient(api_key="key", bearer_token="token")
        assert_credentials_rejected(exc_info)

    @pytest.mark.parametrize(
        "kwargs",
        [{"api_key": ""}, {"bearer_token": "  "}, {"sdk_token": ""}],
    )
    def test_blank_auth_raises_error(self, kwargs):
        """Empty or whitespace-only credentials count as not provided."""
        with pytest.raises(MarketDataError) as exc_info:
            WebSocketClient(**kwargs)
        assert_credentials_rejected(exc_info)

    def test_has_stock_property(self):
        """ws.stock property still works."""
        ws = WebSocketClient(api_key="key")
        assert ws.stock is not None

    def test_has_futopt_property(self):
        """ws.futopt property still works."""
        ws = WebSocketClient(api_key="key")
        assert ws.futopt is not None
