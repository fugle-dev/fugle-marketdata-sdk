package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock intraday endpoints with typed model returns
 *
 * All methods have both async (get_*) and sync (*_sync) variants:
 * - Async methods are preferred for best performance (non-blocking)
 * - Sync methods block the calling thread (simpler API for scripting)
 */
public interface StockIntradayClientInterface {
    
    /**
     * Get candlestick data for a symbol (sync/blocking)
     */
    public String candlesSync(String symbol, String timeframe) throws MarketDataException;
    
    /**
     * Get candlestick data for a symbol (async)
     *
     * timeframe: "1", "5", "10", "15", "30", "60" (minutes)
     * Returns typed IntradayCandlesResponse with OHLCV data.
     */
    public CompletableFuture<String> getCandles(String symbol, String timeframe) ;
    
    /**
     * Get quote for a symbol (async)
     *
     * Returns typed Quote model with all fields directly accessible.
     */
    public CompletableFuture<String> getQuote(String symbol) ;
    
    /**
     * Get ticker info for a symbol (async)
     *
     * Returns typed Ticker model with stock metadata.
     */
    public CompletableFuture<String> getTicker(String symbol) ;
    
    /**
     * Get batch tickers for a security type (async)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    public CompletableFuture<String> getTickers(String typ) ;
    
    /**
     * Get trade history for a symbol (async)
     *
     * Returns typed TradesResponse with list of trades.
     */
    public CompletableFuture<String> getTrades(String symbol) ;
    
    /**
     * Get volume breakdown for a symbol (async)
     *
     * Returns typed VolumesResponse with volume at price data.
     */
    public CompletableFuture<String> getVolumes(String symbol) ;
    
    /**
     * Get quote for a symbol (sync/blocking)
     */
    public String quoteSync(String symbol) throws MarketDataException;
    
    /**
     * Get ticker info for a symbol (sync/blocking)
     */
    public String tickerSync(String symbol) throws MarketDataException;
    
    /**
     * Get batch tickers for a security type (sync/blocking)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    public String tickersSync(String typ) throws MarketDataException;
    
    /**
     * Get trade history for a symbol (sync/blocking)
     */
    public String tradesSync(String symbol) throws MarketDataException;
    
    /**
     * Get volume breakdown for a symbol (sync/blocking)
     */
    public String volumesSync(String symbol) throws MarketDataException;
    
}

