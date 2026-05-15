package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * REST client for UniFFI bindings
 *
 * Wraps the core RestClient and provides Arc-wrapped sub-clients for FFI safety.
 */
public interface RestClientInterface {
    
    /**
     * Access FutOpt (futures and options) endpoints
     */
    public FutOptClient futopt();
    
    /**
     * Access stock-related endpoints
     */
    public StockClient stock();
    
}

