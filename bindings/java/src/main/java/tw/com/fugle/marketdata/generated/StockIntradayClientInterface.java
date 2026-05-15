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
    public IntradayCandlesResponse candlesSync(String symbol, String timeframe) throws MarketDataException;
    
    /**
     * Get candlestick data for a symbol (async)
     *
     * timeframe: "1", "5", "10", "15", "30", "60" (minutes)
     * Returns typed IntradayCandlesResponse with OHLCV data.
     */
    public CompletableFuture<IntradayCandlesResponse> getCandles(String symbol, String timeframe) ;
    
    /**
     * Get quote for a symbol (async)
     *
     * Returns typed Quote model with all fields directly accessible.
     */
    public CompletableFuture<Quote> getQuote(String symbol) ;
    
    /**
     * Get ticker info for a symbol (async)
     *
     * Returns typed Ticker model with stock metadata.
     */
    public CompletableFuture<Ticker> getTicker(String symbol) ;
    
    /**
     * Get batch tickers for a security type (async)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    public CompletableFuture<List<Ticker>> getTickers(String typ) ;
    
    /**
     * Get trade history for a symbol (async)
     *
     * Returns typed TradesResponse with list of trades.
     */
    public CompletableFuture<TradesResponse> getTrades(String symbol) ;
    
    /**
     * Get volume breakdown for a symbol (async)
     *
     * Returns typed VolumesResponse with volume at price data.
     */
    public CompletableFuture<VolumesResponse> getVolumes(String symbol) ;
    
    /**
     * Get quote for a symbol (sync/blocking)
     */
    public Quote quoteSync(String symbol) throws MarketDataException;
    
    /**
     * Get ticker info for a symbol (sync/blocking)
     */
    public Ticker tickerSync(String symbol) throws MarketDataException;
    
    /**
     * Get batch tickers for a security type (sync/blocking)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    public List<Ticker> tickersSync(String typ) throws MarketDataException;
    
    /**
     * Get trade history for a symbol (sync/blocking)
     */
    public TradesResponse tradesSync(String symbol) throws MarketDataException;
    
    /**
     * Get volume breakdown for a symbol (sync/blocking)
     */
    public VolumesResponse volumesSync(String symbol) throws MarketDataException;
    
}

