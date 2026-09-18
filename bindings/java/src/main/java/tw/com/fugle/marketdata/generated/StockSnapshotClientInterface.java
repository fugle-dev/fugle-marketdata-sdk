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
    public String activesSync(String market, String trade) throws MarketDataException;
    
    /**
     * Get most actively traded stocks (async)
     *
     * Parameters:
     * - market: Market code (TSE, OTC)
     * - trade: "volume" or "value" (optional)
     */
    public CompletableFuture<String> getActives(String market, String trade) ;
    
    /**
     * Get the heatmap of an index: its constituents with their change (async)
     *
     * Parameters:
     * - symbol: Index code ("IX0001" for the TAIEX, "IX0027" for the TPEx
     * index). Not a stock symbol or a market: "2330" and "TSE" are 404.
     * - time: Intraday snapshot time, HHmmss (optional; latest by default)
     * - period: Change period instead of the day's change: "1w", "1m", "3m",
     * "6m", "1y", "ytd" (optional)
     */
    public CompletableFuture<String> getHeatmap(String symbol, String time, String period) ;
    
    /**
     * Get top movers (gainers/losers) in a market (async)
     *
     * Parameters:
     * - market: Market code (TSE, OTC)
     * - direction: "up" for gainers, "down" for losers (optional)
     * - change: "percent" or "value" (optional)
     */
    public CompletableFuture<String> getMovers(String market, String direction, String change) ;
    
    /**
     * Get market-wide snapshot quotes (async)
     *
     * Parameters:
     * - market: Market code (TSE, OTC, ESB, TIB, PSB)
     * - type_filter: Optional filter (ALL, ALLBUT0999, COMMONSTOCK)
     */
    public CompletableFuture<String> getQuotes(String market, String typeFilter) ;
    
    /**
     * Get the heatmap of an index (sync/blocking)
     *
     * `symbol` is an index code ("IX0001"), not a stock symbol or a market.
     */
    public String heatmapSync(String symbol, String time, String period) throws MarketDataException;
    
    /**
     * Get top movers (sync/blocking)
     */
    public String moversSync(String market, String direction, String change) throws MarketDataException;
    
    /**
     * Get market-wide snapshot quotes (sync/blocking)
     */
    public String quotesSync(String market, String typeFilter) throws MarketDataException;
    
}

