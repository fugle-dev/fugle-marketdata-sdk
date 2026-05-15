package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
public class MarketdataUniffi {
  
    /**
     * Create a REST client with API key authentication
     *
     * # Arguments
     * * `api_key` - The Fugle API key
     *
     * # Returns
     * A RestClient instance wrapped in Arc for thread-safe access
     */public static RestClient newRestClientWithApiKey(String apiKey) throws MarketDataException {
            try {
                return FfiConverterTypeRestClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key(
            FfiConverterString.INSTANCE.lower(apiKey), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                if (MarketDataException.class.isInstance(_e.getCause())) {
                    throw (MarketDataException)_e.getCause();
                }
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a REST client with API key authentication, custom base URL, and TLS config
     */public static RestClient newRestClientWithApiKeyAndTls(String apiKey, String baseUrl, TlsConfigRecord tls) throws MarketDataException {
            try {
                return FfiConverterTypeRestClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key_and_tls(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterOptionalString.INSTANCE.lower(baseUrl), FfiConverterTypeTlsConfigRecord.INSTANCE.lower(tls), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                if (MarketDataException.class.isInstance(_e.getCause())) {
                    throw (MarketDataException)_e.getCause();
                }
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a REST client with bearer token authentication
     *
     * # Arguments
     * * `bearer_token` - OAuth bearer token
     *
     * # Returns
     * A RestClient instance wrapped in Arc for thread-safe access
     */public static RestClient newRestClientWithBearerToken(String bearerToken) throws MarketDataException {
            try {
                return FfiConverterTypeRestClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token(
            FfiConverterString.INSTANCE.lower(bearerToken), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                if (MarketDataException.class.isInstance(_e.getCause())) {
                    throw (MarketDataException)_e.getCause();
                }
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a REST client with bearer token authentication, custom base URL, and TLS config
     */public static RestClient newRestClientWithBearerTokenAndTls(String bearerToken, String baseUrl, TlsConfigRecord tls) throws MarketDataException {
            try {
                return FfiConverterTypeRestClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token_and_tls(
            FfiConverterString.INSTANCE.lower(bearerToken), FfiConverterOptionalString.INSTANCE.lower(baseUrl), FfiConverterTypeTlsConfigRecord.INSTANCE.lower(tls), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                if (MarketDataException.class.isInstance(_e.getCause())) {
                    throw (MarketDataException)_e.getCause();
                }
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a REST client with SDK token authentication
     *
     * # Arguments
     * * `sdk_token` - Fugle SDK token
     *
     * # Returns
     * A RestClient instance wrapped in Arc for thread-safe access
     */public static RestClient newRestClientWithSdkToken(String sdkToken) throws MarketDataException {
            try {
                return FfiConverterTypeRestClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token(
            FfiConverterString.INSTANCE.lower(sdkToken), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                if (MarketDataException.class.isInstance(_e.getCause())) {
                    throw (MarketDataException)_e.getCause();
                }
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a REST client with SDK token authentication, custom base URL, and TLS config
     */public static RestClient newRestClientWithSdkTokenAndTls(String sdkToken, String baseUrl, TlsConfigRecord tls) throws MarketDataException {
            try {
                return FfiConverterTypeRestClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token_and_tls(
            FfiConverterString.INSTANCE.lower(sdkToken), FfiConverterOptionalString.INSTANCE.lower(baseUrl), FfiConverterTypeTlsConfigRecord.INSTANCE.lower(tls), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                if (MarketDataException.class.isInstance(_e.getCause())) {
                    throw (MarketDataException)_e.getCause();
                }
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a new WebSocket client for stock market data
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     *
     * # Returns
     * A WebSocketClient instance wrapped in Arc for thread-safe access
     */public static WebSocketClient newWebsocketClient(String apiKey, WebSocketListener listener)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_websocket_client(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterTypeWebSocketListener.INSTANCE.lower(listener), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a new WebSocket client with full configuration
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     * * `reconnect_config` - Optional reconnection configuration
     * * `health_check_config` - Optional health check configuration
     *
     * # Returns
     * A WebSocketClient instance wrapped in Arc for thread-safe access
     */public static WebSocketClient newWebsocketClientWithConfig(String apiKey, WebSocketListener listener, WebSocketEndpoint endpoint, ReconnectConfigRecord reconnectConfig, HealthCheckConfigRecord healthCheckConfig)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_websocket_client_with_config(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterTypeWebSocketListener.INSTANCE.lower(listener), FfiConverterTypeWebSocketEndpoint.INSTANCE.lower(endpoint), FfiConverterOptionalTypeReconnectConfigRecord.INSTANCE.lower(reconnectConfig), FfiConverterOptionalTypeHealthCheckConfigRecord.INSTANCE.lower(healthCheckConfig), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Create a new WebSocket client for a specific endpoint
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     *
     * # Returns
     * A WebSocketClient instance wrapped in Arc for thread-safe access
     */public static WebSocketClient newWebsocketClientWithEndpoint(String apiKey, WebSocketListener listener, WebSocketEndpoint endpoint)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_func_new_websocket_client_with_endpoint(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterTypeWebSocketListener.INSTANCE.lower(listener), FfiConverterTypeWebSocketEndpoint.INSTANCE.lower(endpoint), _status);
    })
    );
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

}

