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
 * Stock technical indicator endpoints
 *
 * Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
 * The periods are required by the server and so are positional; the date
 * range is the record.
 */
public class StockTechnicalClient implements AutoCloseable, StockTechnicalClientInterface {
  protected Pointer pointer;
  protected UniffiCleaner.Cleanable cleanable;

  private AtomicBoolean wasDestroyed = new AtomicBoolean(false);
  private AtomicLong callCounter = new AtomicLong(1);

  public StockTechnicalClient(Pointer pointer) {
    this.pointer = pointer;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  /**
   * This constructor can be used to instantiate a fake object. Only used for tests. Any
   * attempt to actually use an object constructed this way will fail as there is no
   * connected Rust object.
   */
  public StockTechnicalClient(NoPointer noPointer) {
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
        throw new IllegalStateException("StockTechnicalClient object has already been destroyed");
      }
      if (c == Long.MAX_VALUE) {
        throw new IllegalStateException("StockTechnicalClient call counter would overflow");
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
          UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_free_stocktechnicalclient(pointer, status);
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
      return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_clone_stocktechnicalclient(pointer, status);
    });
  }

  
    /**
     * Get Bollinger Bands (sync/blocking)
     */
    @Override
    public String bbSync(String symbol, Integer period, TechnicalParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_bb_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(period), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
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
     * Get Bollinger Bands (async)
     */
    @Override
    
    public CompletableFuture<String> getBb(String symbol, Integer period, TechnicalParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_bb(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(period), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterString.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get KDJ (Stochastic Oscillator) (async)
     */
    @Override
    
    public CompletableFuture<String> getKdj(String symbol, Integer rPeriod, Integer kPeriod, Integer dPeriod, TechnicalParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_kdj(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(rPeriod), FfiConverterInteger.INSTANCE.lower(kPeriod), FfiConverterInteger.INSTANCE.lower(dPeriod), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterString.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get MACD indicator (async)
     */
    @Override
    
    public CompletableFuture<String> getMacd(String symbol, Integer fast, Integer slow, Integer signal, TechnicalParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_macd(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(fast), FfiConverterInteger.INSTANCE.lower(slow), FfiConverterInteger.INSTANCE.lower(signal), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterString.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get Relative Strength Index (async)
     */
    @Override
    
    public CompletableFuture<String> getRsi(String symbol, Integer period, TechnicalParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_rsi(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(period), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterString.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get Simple Moving Average (async)
     */
    @Override
    
    public CompletableFuture<String> getSma(String symbol, Integer period, TechnicalParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_get_sma(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(period), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterString.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get KDJ (sync/blocking)
     */
    @Override
    public String kdjSync(String symbol, Integer rPeriod, Integer kPeriod, Integer dPeriod, TechnicalParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_kdj_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(rPeriod), FfiConverterInteger.INSTANCE.lower(kPeriod), FfiConverterInteger.INSTANCE.lower(dPeriod), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
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
     * Get MACD (sync/blocking)
     */
    @Override
    public String macdSync(String symbol, Integer fast, Integer slow, Integer signal, TechnicalParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_macd_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(fast), FfiConverterInteger.INSTANCE.lower(slow), FfiConverterInteger.INSTANCE.lower(signal), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
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
     * Get Relative Strength Index (sync/blocking)
     */
    @Override
    public String rsiSync(String symbol, Integer period, TechnicalParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_rsi_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(period), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
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
     * Get Simple Moving Average (sync/blocking)
     */
    @Override
    public String smaSync(String symbol, Integer period, TechnicalParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stocktechnicalclient_sma_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterInteger.INSTANCE.lower(period), FfiConverterOptionalTypeTechnicalParams.INSTANCE.lower(params), _status);
    });
    
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
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
    

  

  
}



