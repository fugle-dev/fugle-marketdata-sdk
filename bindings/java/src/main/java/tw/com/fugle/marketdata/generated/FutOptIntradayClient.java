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
 * FutOpt intraday endpoints
 */
public class FutOptIntradayClient implements AutoCloseable, FutOptIntradayClientInterface {
  protected Pointer pointer;
  protected UniffiCleaner.Cleanable cleanable;

  private AtomicBoolean wasDestroyed = new AtomicBoolean(false);
  private AtomicLong callCounter = new AtomicLong(1);

  public FutOptIntradayClient(Pointer pointer) {
    this.pointer = pointer;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  /**
   * This constructor can be used to instantiate a fake object. Only used for tests. Any
   * attempt to actually use an object constructed this way will fail as there is no
   * connected Rust object.
   */
  public FutOptIntradayClient(NoPointer noPointer) {
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
        throw new IllegalStateException("FutOptIntradayClient object has already been destroyed");
      }
      if (c == Long.MAX_VALUE) {
        throw new IllegalStateException("FutOptIntradayClient call counter would overflow");
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
          UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_free_futoptintradayclient(pointer, status);
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
      return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_clone_futoptintradayclient(pointer, status);
    });
  }

  
    /**
     * Get candlestick data for a contract (sync/blocking)
     */
    @Override
    public String candlesSync(String symbol, FutOptCandlesParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_candles_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeFutOptCandlesParams.INSTANCE.lower(params), _status);
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
     * Get candlestick data for a futures/options contract (async)
     */
    @Override
    
    public CompletableFuture<String> getCandles(String symbol, FutOptCandlesParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_candles(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeFutOptCandlesParams.INSTANCE.lower(params)
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
     * Get available products list (async)
     *
     * typ: "F" for futures, "O" for options
     */
    @Override
    
    public CompletableFuture<String> getProducts(String typ, FutOptProductsParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_products(
                thisPtr,
                FfiConverterString.INSTANCE.lower(typ), FfiConverterOptionalTypeFutOptProductsParams.INSTANCE.lower(params)
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
     * Get quote for a futures/options contract (async)
     */
    @Override
    
    public CompletableFuture<String> getQuote(String symbol, AfterHoursParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_quote(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeAfterHoursParams.INSTANCE.lower(params)
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
     * Get ticker info for a contract (async)
     */
    @Override
    
    public CompletableFuture<String> getTicker(String symbol, AfterHoursParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_ticker(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeAfterHoursParams.INSTANCE.lower(params)
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
     * Get batch tickers for futures/options (async)
     *
     * typ: "F" for futures, "O" for options
     */
    @Override
    
    public CompletableFuture<String> getTickers(String typ, FutOptTickersParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_tickers(
                thisPtr,
                FfiConverterString.INSTANCE.lower(typ), FfiConverterOptionalTypeFutOptTickersParams.INSTANCE.lower(params)
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
     * Get trade history for a futures/options contract (async)
     */
    @Override
    
    public CompletableFuture<String> getTrades(String symbol, FutOptTradesParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_trades(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeFutOptTradesParams.INSTANCE.lower(params)
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
     * Get volume breakdown by price for a futures/options contract (async)
     */
    @Override
    
    public CompletableFuture<String> getVolumes(String symbol, AfterHoursParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_get_volumes(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeAfterHoursParams.INSTANCE.lower(params)
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
     * Get available products list (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    @Override
    public String productsSync(String typ, FutOptProductsParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_products_sync(
            it, FfiConverterString.INSTANCE.lower(typ), FfiConverterOptionalTypeFutOptProductsParams.INSTANCE.lower(params), _status);
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
     * Get quote for a futures/options contract (sync/blocking)
     */
    @Override
    public String quoteSync(String symbol, AfterHoursParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_quote_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeAfterHoursParams.INSTANCE.lower(params), _status);
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
     * Get ticker info for a contract (sync/blocking)
     */
    @Override
    public String tickerSync(String symbol, AfterHoursParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_ticker_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeAfterHoursParams.INSTANCE.lower(params), _status);
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
     * Get batch tickers for futures/options (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    @Override
    public String tickersSync(String typ, FutOptTickersParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_tickers_sync(
            it, FfiConverterString.INSTANCE.lower(typ), FfiConverterOptionalTypeFutOptTickersParams.INSTANCE.lower(params), _status);
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
     * Get trade history for a contract (sync/blocking)
     */
    @Override
    public String tradesSync(String symbol, FutOptTradesParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_trades_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeFutOptTradesParams.INSTANCE.lower(params), _status);
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
     * Get volume breakdown by price for a contract (sync/blocking)
     */
    @Override
    public String volumesSync(String symbol, AfterHoursParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_futoptintradayclient_volumes_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeAfterHoursParams.INSTANCE.lower(params), _status);
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



