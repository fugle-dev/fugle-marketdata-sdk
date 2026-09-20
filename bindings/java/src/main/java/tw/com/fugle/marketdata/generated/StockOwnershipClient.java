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
 * Stock ownership endpoints client
 */
public class StockOwnershipClient implements AutoCloseable, StockOwnershipClientInterface {
  protected Pointer pointer;
  protected UniffiCleaner.Cleanable cleanable;

  private AtomicBoolean wasDestroyed = new AtomicBoolean(false);
  private AtomicLong callCounter = new AtomicLong(1);

  public StockOwnershipClient(Pointer pointer) {
    this.pointer = pointer;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  /**
   * This constructor can be used to instantiate a fake object. Only used for tests. Any
   * attempt to actually use an object constructed this way will fail as there is no
   * connected Rust object.
   */
  public StockOwnershipClient(NoPointer noPointer) {
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
        throw new IllegalStateException("StockOwnershipClient object has already been destroyed");
      }
      if (c == Long.MAX_VALUE) {
        throw new IllegalStateException("StockOwnershipClient call counter would overflow");
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
          UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_free_stockownershipclient(pointer, status);
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
      return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_clone_stockownershipclient(pointer, status);
    });
  }

  
    /**
     * Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
     */
    @Override
    public String directorHoldingsSync(String symbol, OwnershipParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_director_holdings_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params), _status);
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
     * Get the constituents an ETF held over a date range (sync/blocking)
     */
    @Override
    public String etfHoldingsSync(String symbol, OwnershipParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_etf_holdings_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params), _status);
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
     * Get monthly holdings and pledges disclosed by directors and supervisors (async)
     */
    @Override
    
    public CompletableFuture<String> getDirectorHoldings(String symbol, OwnershipParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_director_holdings(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params)
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
     * Get the constituents an ETF held over a date range (async)
     */
    @Override
    
    public CompletableFuture<String> getEtfHoldings(String symbol, OwnershipParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_etf_holdings(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params)
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
     * Get daily trading by the three major institutional investors (async)
     */
    @Override
    
    public CompletableFuture<String> getInstitutionalTrades(String symbol, OwnershipParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_institutional_trades(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params)
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
     * Get the weekly TDCC shareholder distribution by holding-size bracket (async)
     */
    @Override
    
    public CompletableFuture<String> getTdccDistribution(String symbol, OwnershipParams params){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_get_tdcc_distribution(
                thisPtr,
                FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params)
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
     * Get daily trading by the three major institutional investors (sync/blocking)
     */
    @Override
    public String institutionalTradesSync(String symbol, OwnershipParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_institutional_trades_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params), _status);
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
     * Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
     */
    @Override
    public String tdccDistributionSync(String symbol, OwnershipParams params) throws MarketDataException {
            try {
                return FfiConverterString.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockownershipclient_tdcc_distribution_sync(
            it, FfiConverterString.INSTANCE.lower(symbol), FfiConverterOptionalTypeOwnershipParams.INSTANCE.lower(params), _status);
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



