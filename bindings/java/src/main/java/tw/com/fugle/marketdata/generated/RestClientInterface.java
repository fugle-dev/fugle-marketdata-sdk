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
     * The prefix every request from this client is built on, fully resolved —
     * host, path prefix and version segment.
     *
     * The version segment is chosen by the SDK rather than written by the
     * caller, so this is the only way to see what a client resolved to.
     */
    public String baseUrl();
    
    /**
     * Access FutOpt (futures and options) endpoints
     */
    public FutOptClient futopt();
    
    /**
     * Replace the credential later requests send, with any of the three
     * kinds. Sub-clients already taken from this one (`stock()`,
     * `stock().intraday()`, ...) send it too; a request already sent keeps
     * the credential it was sent with (#322).
     *
     * Exactly one credential must be non-empty, as at construction;
     * otherwise, or if it cannot be sent in an HTTP header, this returns a
     * `ConfigError` (code 1004) and the current credential is kept.
     */
    public void setCredentials(CredentialsRecord credentials) throws MarketDataException;
    
    /**
     * Access stock-related endpoints
     */
    public StockClient stock();
    
}

