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
    public String activesSync(String market, String trade, SnapshotParams params) throws MarketDataException;
    
    /**
     * Get most actively traded stocks (async)
     *
     * trade: "volume" or "value"
     */
    public CompletableFuture<String> getActives(String market, String trade, SnapshotParams params) ;
    
    /**
     * Get top movers (gainers/losers) in a market (async)
     *
     * direction: "up" for gainers, "down" for losers;
     * change: "percent" or "value"
     */
    public CompletableFuture<String> getMovers(String market, String direction, String change, MoversParams params) ;
    
    /**
     * Get market-wide snapshot quotes (async)
     *
     * market: TSE, OTC, ESB, TIB or PSB
     */
    public CompletableFuture<String> getQuotes(String market, SnapshotParams params) ;
    
    /**
     * Get top movers (sync/blocking)
     */
    public String moversSync(String market, String direction, String change, MoversParams params) throws MarketDataException;
    
    /**
     * Get market-wide snapshot quotes (sync/blocking)
     */
    public String quotesSync(String market, SnapshotParams params) throws MarketDataException;
    
}

