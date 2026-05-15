package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock historical endpoints with typed model returns
 *
 * All methods have both async (get_*) and sync (*_sync) variants:
 * - Async methods are preferred for best performance (non-blocking)
 * - Sync methods block the calling thread (simpler API for scripting)
 */
public interface StockHistoricalClientInterface {
    
    /**
     * Get historical candles for a symbol (sync/blocking)
     */
    public HistoricalCandlesResponse candlesSync(String symbol, String from, String to, String timeframe) throws MarketDataException;
    
    /**
     * Get historical candles for a symbol (async)
     *
     * Parameters:
     * - symbol: Stock symbol (e.g., "2330")
     * - from: Start date (YYYY-MM-DD, optional)
     * - to: End date (YYYY-MM-DD, optional)
     * - timeframe: "D" (day), "W" (week), "M" (month), or intraday "1", "5", "10", "15", "30", "60"
     */
    public CompletableFuture<HistoricalCandlesResponse> getCandles(String symbol, String from, String to, String timeframe) ;
    
    /**
     * Get historical stats for a symbol (async)
     *
     * Returns summary statistics including 52-week high/low
     */
    public CompletableFuture<StatsResponse> getStats(String symbol) ;
    
    /**
     * Get historical stats for a symbol (sync/blocking)
     */
    public StatsResponse statsSync(String symbol) throws MarketDataException;
    
}

