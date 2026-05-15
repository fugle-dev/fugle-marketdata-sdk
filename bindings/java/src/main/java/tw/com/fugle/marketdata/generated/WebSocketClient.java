package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Function;
import java.util.function.Consumer;
import com.sun.jna.Pointer;
import java.util.concurrent.CompletableFuture;
/**
 * WebSocket client for real-time market data streaming
 *
 * Wraps the core WebSocketClient and forwards messages to the provided
 * WebSocketListener implementation via a background task.
 */
public class WebSocketClient implements AutoCloseable, WebSocketClientInterface {
  protected Pointer pointer;
  protected UniffiCleaner.Cleanable cleanable;

  private AtomicBoolean wasDestroyed = new AtomicBoolean(false);
  private AtomicLong callCounter = new AtomicLong(1);

  public WebSocketClient(Pointer pointer) {
    this.pointer = pointer;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  /**
   * This constructor can be used to instantiate a fake object. Only used for tests. Any
   * attempt to actually use an object constructed this way will fail as there is no
   * connected Rust object.
   */
  public WebSocketClient(NoPointer noPointer) {
    this.pointer = null;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  
    /**
     * Create a new WebSocket client for stock market data
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     */
  public WebSocketClient(String apiKey, WebSocketListener listener) {
    this((Pointer)
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterTypeWebSocketListener.INSTANCE.lower(listener), _status);
    })
    );
  }

  @Override
  public synchronized void close() {
    // Only allow a single call to this method.
    // TODO(uniffi): maybe we should log a warning if called more than once?
    if (this.wasDestroyed.compareAndSet(false, true)) {
      // This decrement always matches the initial count of 1 given at creation time.
      if (this.callCounter.decrementAndGet() == 0L) {
        cleanable.clean();
      }
    }
  }

  public <R> R callWithPointer(Function<Pointer, R> block) {
    // Check and increment the call counter, to keep the object alive.
    // This needs a compare-and-set retry loop in case of concurrent updates.
    long c;
    do {
      c = this.callCounter.get();
      if (c == 0L) {
        throw new IllegalStateException("WebSocketClient object has already been destroyed");
      }
      if (c == Long.MAX_VALUE) {
        throw new IllegalStateException("WebSocketClient call counter would overflow");
      }
    } while (! this.callCounter.compareAndSet(c, c + 1L));
    // Now we can safely do the method call without the pointer being freed concurrently.
    try {
      return block.apply(this.uniffiClonePointer());
    } finally {
      // This decrement always matches the increment we performed above.
      if (this.callCounter.decrementAndGet() == 0L) {
          cleanable.clean();
      }
    }
  }

  public void callWithPointer(Consumer<Pointer> block) {
    callWithPointer((Pointer p) -> {
      block.accept(p);
      return (Void)null;
    });
  }

  private class UniffiCleanAction implements Runnable {
    private final Pointer pointer;

    public UniffiCleanAction(Pointer pointer) {
      this.pointer = pointer;
    }

    @Override
    public void run() {
      if (pointer != null) {
        UniffiHelpers.uniffiRustCall(status -> {
          UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_free_websocketclient(pointer, status);
          return null;
        });
      }
    }
  }

  Pointer uniffiClonePointer() {
    return UniffiHelpers.uniffiRustCall(status -> {
      if (pointer == null) {
        throw new NullPointerException();
      }
      return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_clone_websocketclient(pointer, status);
    });
  }

  
    @Override
    
    public CompletableFuture<Void> connect(){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_connect(
                thisPtr
                
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_void(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_void(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_void(future),
        // lift function
        () -> {},
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    @Override
    
    public CompletableFuture<Void> disconnect(){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_disconnect(
                thisPtr
                
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_void(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_void(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_void(future),
        // lift function
        () -> {},
        // Error FFI converter
        new UniffiNullRustCallStatusErrorHandler()
    );
    }

  
    /**
     * Check if the client has been shut down
     */
    @Override
    public Boolean isClosed()  {
            try {
                return FfiConverterBoolean.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_is_closed(
            it, _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
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
     * Check if the client is currently connected
     */
    @Override
    public Boolean isConnected()  {
            try {
                return FfiConverterBoolean.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_is_connected(
            it, _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    );
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    @Override
    
    public CompletableFuture<Void> ping(String state){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_ping(
                thisPtr,
                FfiConverterOptionalString.INSTANCE.lower(state)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_void(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_void(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_void(future),
        // lift function
        () -> {},
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    @Override
    
    public CompletableFuture<Void> querySubscriptions(){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_query_subscriptions(
                thisPtr
                
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_void(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_void(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_void(future),
        // lift function
        () -> {},
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    @Override
    
    public CompletableFuture<Void> subscribe(String channel, String symbol){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_subscribe(
                thisPtr,
                FfiConverterString.INSTANCE.lower(channel), FfiConverterString.INSTANCE.lower(symbol)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_void(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_void(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_void(future),
        // lift function
        () -> {},
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    @Override
    
    public CompletableFuture<Void> unsubscribe(String channel, String symbol){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketclient_unsubscribe(
                thisPtr,
                FfiConverterString.INSTANCE.lower(channel), FfiConverterString.INSTANCE.lower(symbol)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_void(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_void(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_void(future),
        // lift function
        () -> {},
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
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
     */public static WebSocketClient newWithConfig(String apiKey, WebSocketListener listener, WebSocketEndpoint endpoint, ReconnectConfigRecord reconnectConfig, HealthCheckConfigRecord healthCheckConfig)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_config(
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
     */public static WebSocketClient newWithEndpoint(String apiKey, WebSocketListener listener, WebSocketEndpoint endpoint)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_endpoint(
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
    

  
    /**
     * Create a new WebSocket client with full configuration including TLS.
     *
     * All optional parameters can be None to use defaults. This is the
     * TLS-aware variant of `new_with_url` — use this when you need to
     * pin a custom CA or disable cert verification.
     *
     * # Arguments
     * * `api_key` - Fugle API key for authentication
     * * `listener` - Callback interface for receiving WebSocket events
     * * `endpoint` - The market data endpoint (Stock or FutOpt)
     * * `base_url` - Optional base URL override
     * * `reconnect_config` - Optional reconnection configuration
     * * `health_check_config` - Optional health check configuration
     * * `tls` - Optional TLS customization (custom CA or accept_invalid_certs)
     */public static WebSocketClient newWithFullConfig(String apiKey, WebSocketListener listener, WebSocketEndpoint endpoint, String baseUrl, ReconnectConfigRecord reconnectConfig, HealthCheckConfigRecord healthCheckConfig, TlsConfigRecord tls)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_full_config(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterTypeWebSocketListener.INSTANCE.lower(listener), FfiConverterTypeWebSocketEndpoint.INSTANCE.lower(endpoint), FfiConverterOptionalString.INSTANCE.lower(baseUrl), FfiConverterOptionalTypeReconnectConfigRecord.INSTANCE.lower(reconnectConfig), FfiConverterOptionalTypeHealthCheckConfigRecord.INSTANCE.lower(healthCheckConfig), FfiConverterOptionalTypeTlsConfigRecord.INSTANCE.lower(tls), _status);
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
     * Create a new WebSocket client with full configuration including custom base URL
     */public static WebSocketClient newWithUrl(String apiKey, WebSocketListener listener, WebSocketEndpoint endpoint, String baseUrl, ReconnectConfigRecord reconnectConfig, HealthCheckConfigRecord healthCheckConfig)  {
            try {
                return FfiConverterTypeWebSocketClient.INSTANCE.lift(
    UniffiHelpers.uniffiRustCall( _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_url(
            FfiConverterString.INSTANCE.lower(apiKey), FfiConverterTypeWebSocketListener.INSTANCE.lower(listener), FfiConverterTypeWebSocketEndpoint.INSTANCE.lower(endpoint), FfiConverterString.INSTANCE.lower(baseUrl), FfiConverterOptionalTypeReconnectConfigRecord.INSTANCE.lower(reconnectConfig), FfiConverterOptionalTypeHealthCheckConfigRecord.INSTANCE.lower(healthCheckConfig), _status);
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



