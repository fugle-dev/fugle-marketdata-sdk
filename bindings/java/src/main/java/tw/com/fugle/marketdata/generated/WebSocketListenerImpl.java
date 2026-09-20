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
 * Callback interface for WebSocket events
 *
 * Foreign code (C#, Go) implements this trait to receive WebSocket events.
 * The implementation must be thread-safe (Send + Sync) as callbacks may be
 * invoked from background tokio tasks.
 *
 * # Example (C#)
 *
 * ```csharp
 * class MyListener : IWebSocketListener {
 * public void OnConnected() {
 * Console.WriteLine("Connected!");
 * }
 * public void OnAuthenticated(string? dataJson) {
 * Console.WriteLine("Authenticated");
 * }
 * public void OnUnauthenticated(string? dataJson) {
 * Console.WriteLine($"Rejected: {dataJson}");
 * }
 * public void OnDisconnected(bool willReconnect) {
 * Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(ErrorInfo error) {
 * Console.WriteLine($"Error: {error.Message}");
 * }
 * }
 * ```
 */
public class WebSocketListenerImpl implements AutoCloseable, WebSocketListener {
  protected Pointer pointer;
  protected UniffiCleaner.Cleanable cleanable;

  private AtomicBoolean wasDestroyed = new AtomicBoolean(false);
  private AtomicLong callCounter = new AtomicLong(1);

  public WebSocketListenerImpl(Pointer pointer) {
    this.pointer = pointer;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  /**
   * This constructor can be used to instantiate a fake object. Only used for tests. Any
   * attempt to actually use an object constructed this way will fail as there is no
   * connected Rust object.
   */
  public WebSocketListenerImpl(NoPointer noPointer) {
    this.pointer = null;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
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
        throw new IllegalStateException("WebSocketListenerImpl object has already been destroyed");
      }
      if (c == Long.MAX_VALUE) {
        throw new IllegalStateException("WebSocketListenerImpl call counter would overflow");
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
          UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_free_websocketlistener(pointer, status);
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
      return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_clone_websocketlistener(pointer, status);
    });
  }

  
    /**
     * Called when the transport is established, before the server has
     * answered the auth frame. Fires again on every successful reconnect.
     * Wait for `on_authenticated` before treating the connection as usable.
     */
    @Override
    public void onConnected()  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_connected(
            it, _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when the server accepts the credentials.
     *
     * `data_json` is the `data` member of the server's `authenticated`
     * frame, still encoded as JSON, or `None` when the frame has none.
     */
    @Override
    public void onAuthenticated(String dataJson)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_authenticated(
            it, FfiConverterOptionalString.INSTANCE.lower(dataJson), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when the server rejects the credentials: it answered the auth
     * frame with an `error` of code 1000. On `connect()` the call also
     * fails with an auth error; no `on_error` is emitted for the rejection.
     * During an auto-reconnect, `on_reconnect_failed` follows at once: the
     * same credentials would be rejected again, so the client stops and
     * stays closed (#201). An auth-phase `error` with any other code (1011
     * auth service unavailable, 1004 no auth request received) is not a
     * rejection: it is reported to `on_error` (code 2001) and a reconnect
     * goes on.
     *
     * `data_json` is the `data` member of the server's rejection frame
     * (the server's message is under `message`), still encoded as JSON, or
     * `None` when the frame has none.
     */
    @Override
    public void onUnauthenticated(String dataJson)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_unauthenticated(
            it, FfiConverterOptionalString.INSTANCE.lower(dataJson), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when the connection is closed, at most once per connection.
     *
     * `will_reconnect` is `true` when the client will try to reconnect
     * (`on_reconnecting` follows unless `disconnect()` is called first) and
     * `false` when this connection's lifecycle has ended.
     */
    @Override
    public void onDisconnected(Boolean willReconnect)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_disconnected(
            it, FfiConverterBoolean.INSTANCE.lower(willReconnect), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when a message is received
     */
    @Override
    public void onMessage(StreamMessage message)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_message(
            it, FfiConverterTypeStreamMessage.INSTANCE.lower(message), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when an error occurs
     */
    @Override
    public void onError(ErrorInfo error)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_error(
            it, FfiConverterTypeErrorInfo.INSTANCE.lower(error), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when a reconnection attempt starts
     */
    @Override
    public void onReconnecting(Integer attempt)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnecting(
            it, FfiConverterInteger.INSTANCE.lower(attempt), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when the reconnect gives up: all attempts are exhausted, or an
     * attempt's credentials were rejected (`on_unauthenticated` precedes
     * it, #201). Terminal: no further lifecycle callbacks follow for this
     * connection.
     */
    @Override
    public void onReconnectFailed(Integer attempts)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnect_failed(
            it, FfiConverterInteger.INSTANCE.lower(attempts), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  
    /**
     * Called when messages were dropped because `on_message` fell behind
     * while the client's message queue held `buffer` unread messages
     * (`MessageOverflowRecord::DropNewest`).
     *
     * `count` is the number dropped since the previous call. The first drop
     * on a connection is reported at once, later ones at most once per
     * second, and the rest before `on_disconnected`. The connection's total
     * is `WebSocketClient::messages_dropped_total()`.
     */
    @Override
    public void onMessagesDropped(Long count)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_messages_dropped(
            it, FfiConverterLong.INSTANCE.lower(count), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    })
    ;
            } catch (RuntimeException _e) {
                
                
                if (InternalException.class.isInstance(_e.getCause())) {
                    throw (InternalException)_e.getCause();
                }
                throw _e;
            }
    }
    

  

  
}


