package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock historical endpoints
 */
public interface StockHistoricalClientInterface {
    
    /**
     * Get historical candles for a symbol (sync/blocking)
     */
    public String candlesSync(String symbol, StockHistoricalCandlesParams params) throws MarketDataException;
    
    /**
     * Get historical candles for a symbol (async)
     */
    public CompletableFuture<String> getCandles(String symbol, StockHistoricalCandlesParams params) ;
    
    /**
     * Get historical stats for a symbol (async)
     *
     * Returns summary statistics including 52-week high/low
     */
    public CompletableFuture<String> getStats(String symbol) ;
    
    /**
     * Get historical stats for a symbol (sync/blocking)
     */
    public String statsSync(String symbol) throws MarketDataException;
    
}

