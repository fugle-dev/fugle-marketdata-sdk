package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock intraday endpoints
 *
 * All methods have both async (get_*) and sync (*_sync) variants:
 * - Async methods are preferred for best performance (non-blocking)
 * - Sync methods block the calling thread (simpler API for scripting)
 */
public interface StockIntradayClientInterface {
    
    /**
     * Get candlestick data for a symbol (sync/blocking)
     */
    public String candlesSync(String symbol, StockCandlesParams params) throws MarketDataException;
    
    /**
     * Get candlestick data for a symbol (async)
     *
     * `timeframe` is in the record: unset takes the server default.
     */
    public CompletableFuture<String> getCandles(String symbol, StockCandlesParams params) ;
    
    /**
     * Get quote for a symbol (async)
     */
    public CompletableFuture<String> getQuote(String symbol, OddLotParams params) ;
    
    /**
     * Get ticker info for a symbol (async)
     */
    public CompletableFuture<String> getTicker(String symbol, OddLotParams params) ;
    
    /**
     * Get batch tickers for a security type (async)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    public CompletableFuture<String> getTickers(String typ, StockTickersParams params) ;
    
    /**
     * Get trade history for a symbol (async)
     */
    public CompletableFuture<String> getTrades(String symbol, StockTradesParams params) ;
    
    /**
     * Get volume breakdown for a symbol (async)
     */
    public CompletableFuture<String> getVolumes(String symbol, OddLotParams params) ;
    
    /**
     * Get quote for a symbol (sync/blocking)
     */
    public String quoteSync(String symbol, OddLotParams params) throws MarketDataException;
    
    /**
     * Get ticker info for a symbol (sync/blocking)
     */
    public String tickerSync(String symbol, OddLotParams params) throws MarketDataException;
    
    /**
     * Get batch tickers for a security type (sync/blocking)
     *
     * typ: Security type (e.g., "EQUITY", "INDEX", "ETF")
     */
    public String tickersSync(String typ, StockTickersParams params) throws MarketDataException;
    
    /**
     * Get trade history for a symbol (sync/blocking)
     */
    public String tradesSync(String symbol, StockTradesParams params) throws MarketDataException;
    
    /**
     * Get volume breakdown for a symbol (sync/blocking)
     */
    public String volumesSync(String symbol, OddLotParams params) throws MarketDataException;
    
}

