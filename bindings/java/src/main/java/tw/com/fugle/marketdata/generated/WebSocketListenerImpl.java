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
 * public void OnDisconnected() {
 * Console.WriteLine("Disconnected");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(string errorMessage) {
 * Console.WriteLine($"Error: {errorMessage}");
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
     * Called when WebSocket connection is established
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
     * Called when WebSocket connection is closed
     */
    @Override
    public void onDisconnected()  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_disconnected(
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
    public void onError(String errorMessage)  {
            try {
                
    callWithPointer(it -> {
        try {
    
    UniffiHelpers.uniffiRustCall( _status -> {
        UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_websocketlistener_on_error(
            it, FfiConverterString.INSTANCE.lower(errorMessage), _status);
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
     * Called when all reconnection attempts are exhausted
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
    

  

  
}


