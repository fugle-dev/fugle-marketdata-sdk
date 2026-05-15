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
 * Stock corporate actions endpoints
 *
 * Provides access to capital changes, dividends, and listing applicants (IPO).
 */
public class StockCorporateActionsClient implements AutoCloseable, StockCorporateActionsClientInterface {
  protected Pointer pointer;
  protected UniffiCleaner.Cleanable cleanable;

  private AtomicBoolean wasDestroyed = new AtomicBoolean(false);
  private AtomicLong callCounter = new AtomicLong(1);

  public StockCorporateActionsClient(Pointer pointer) {
    this.pointer = pointer;
    this.cleanable = UniffiLib.CLEANER.register(this, new UniffiCleanAction(pointer));
  }

  /**
   * This constructor can be used to instantiate a fake object. Only used for tests. Any
   * attempt to actually use an object constructed this way will fail as there is no
   * connected Rust object.
   */
  public StockCorporateActionsClient(NoPointer noPointer) {
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
        throw new IllegalStateException("StockCorporateActionsClient object has already been destroyed");
      }
      if (c == Long.MAX_VALUE) {
        throw new IllegalStateException("StockCorporateActionsClient call counter would overflow");
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
          UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_free_stockcorporateactionsclient(pointer, status);
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
      return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_clone_stockcorporateactionsclient(pointer, status);
    });
  }

  
    /**
     * Get capital structure changes (sync/blocking)
     */
    @Override
    public CapitalChangesResponse capitalChangesSync(String date, String startDate, String endDate) throws MarketDataException {
            try {
                return FfiConverterTypeCapitalChangesResponse.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_capital_changes_sync(
            it, FfiConverterOptionalString.INSTANCE.lower(date), FfiConverterOptionalString.INSTANCE.lower(startDate), FfiConverterOptionalString.INSTANCE.lower(endDate), _status);
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
     * Get dividend announcements (sync/blocking)
     */
    @Override
    public DividendsResponse dividendsSync(String date, String startDate, String endDate) throws MarketDataException {
            try {
                return FfiConverterTypeDividendsResponse.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_dividends_sync(
            it, FfiConverterOptionalString.INSTANCE.lower(date), FfiConverterOptionalString.INSTANCE.lower(startDate), FfiConverterOptionalString.INSTANCE.lower(endDate), _status);
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
     * Get capital structure changes (async)
     */
    @Override
    
    public CompletableFuture<CapitalChangesResponse> getCapitalChanges(String date, String startDate, String endDate){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_capital_changes(
                thisPtr,
                FfiConverterOptionalString.INSTANCE.lower(date), FfiConverterOptionalString.INSTANCE.lower(startDate), FfiConverterOptionalString.INSTANCE.lower(endDate)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterTypeCapitalChangesResponse.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get dividend announcements (async)
     */
    @Override
    
    public CompletableFuture<DividendsResponse> getDividends(String date, String startDate, String endDate){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_dividends(
                thisPtr,
                FfiConverterOptionalString.INSTANCE.lower(date), FfiConverterOptionalString.INSTANCE.lower(startDate), FfiConverterOptionalString.INSTANCE.lower(endDate)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterTypeDividendsResponse.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get IPO listing applicants (async)
     */
    @Override
    
    public CompletableFuture<ListingApplicantsResponse> getListingApplicants(String date, String startDate, String endDate){
        return UniffiAsyncHelpers.uniffiRustCallAsync(
        callWithPointer(thisPtr -> {
            return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_get_listing_applicants(
                thisPtr,
                FfiConverterOptionalString.INSTANCE.lower(date), FfiConverterOptionalString.INSTANCE.lower(startDate), FfiConverterOptionalString.INSTANCE.lower(endDate)
            );
        }),
        (future, callback, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_poll_rust_buffer(future, callback, continuation),
        (future, continuation) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_complete_rust_buffer(future, continuation),
        (future) -> UniffiLib.INSTANCE.ffi_marketdata_uniffi_rust_future_free_rust_buffer(future),
        // lift function
        (it) -> FfiConverterTypeListingApplicantsResponse.INSTANCE.lift(it),
        // Error FFI converter
        new MarketDataExceptionErrorHandler()
    );
    }

  
    /**
     * Get IPO listing applicants (sync/blocking)
     */
    @Override
    public ListingApplicantsResponse listingApplicantsSync(String date, String startDate, String endDate) throws MarketDataException {
            try {
                return FfiConverterTypeListingApplicantsResponse.INSTANCE.lift(
    callWithPointer(it -> {
        try {
    
            return
    UniffiHelpers.uniffiRustCallWithError(new MarketDataExceptionErrorHandler(), _status -> {
        return UniffiLib.INSTANCE.uniffi_marketdata_uniffi_fn_method_stockcorporateactionsclient_listing_applicants_sync(
            it, FfiConverterOptionalString.INSTANCE.lower(date), FfiConverterOptionalString.INSTANCE.lower(startDate), FfiConverterOptionalString.INSTANCE.lower(endDate), _status);
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



