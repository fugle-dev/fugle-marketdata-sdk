package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock snapshot endpoints for market-wide data
 *
 * Provides access to quotes, movers (gainers/losers), and most active stocks
 * across entire markets.
 */
public interface StockSnapshotClientInterface {
    
    /**
     * Get most actively traded stocks (sync/blocking)
     */
    public ActivesResponse activesSync(String market, String trade) throws MarketDataException;
    
    /**
     * Get most actively traded stocks (async)
     *
     * Parameters:
     * - market: Market code (TSE, OTC)
     * - trade: "volume" or "value" (optional)
     */
    public CompletableFuture<ActivesResponse> getActives(String market, String trade) ;
    
    /**
     * Get top movers (gainers/losers) in a market (async)
     *
     * Parameters:
     * - market: Market code (TSE, OTC)
     * - direction: "up" for gainers, "down" for losers (optional)
     * - change: "percent" or "value" (optional)
     */
    public CompletableFuture<MoversResponse> getMovers(String market, String direction, String change) ;
    
    /**
     * Get market-wide snapshot quotes (async)
     *
     * Parameters:
     * - market: Market code (TSE, OTC, ESB, TIB, PSB)
     * - type_filter: Optional filter (ALL, ALLBUT0999, COMMONSTOCK)
     */
    public CompletableFuture<SnapshotQuotesResponse> getQuotes(String market, String typeFilter) ;
    
    /**
     * Get top movers (sync/blocking)
     */
    public MoversResponse moversSync(String market, String direction, String change) throws MarketDataException;
    
    /**
     * Get market-wide snapshot quotes (sync/blocking)
     */
    public SnapshotQuotesResponse quotesSync(String market, String typeFilter) throws MarketDataException;
    
}

