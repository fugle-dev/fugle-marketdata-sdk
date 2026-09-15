package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock market data client
 */
public interface StockClientInterface {
    
    /**
     * The fully resolved request prefix for this product client.
     */
    public String baseUrl();
    
    /**
     * Access corporate actions endpoints
     */
    public StockCorporateActionsClient corporateActions();
    
    /**
     * Access historical data endpoints
     */
    public StockHistoricalClient historical();
    
    /**
     * Access intraday (real-time) endpoints
     */
    public StockIntradayClient intraday();
    
    /**
     * Access ownership endpoints (ETF holdings, institutional trades, director
     * holdings, TDCC distribution)
     */
    public StockOwnershipClient ownership();
    
    /**
     * Access snapshot (market-wide) endpoints
     */
    public StockSnapshotClient snapshot();
    
    /**
     * Access technical indicator endpoints
     */
    public StockTechnicalClient technical();
    
}

