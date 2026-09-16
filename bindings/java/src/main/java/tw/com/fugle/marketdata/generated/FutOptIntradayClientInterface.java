package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * FutOpt intraday endpoints with typed model returns
 */
public interface FutOptIntradayClientInterface {
    
    /**
     * Get candlestick data for a contract (sync/blocking)
     */
    public String candlesSync(String symbol, String timeframe) throws MarketDataException;
    
    /**
     * Get candlestick data for a futures/options contract (async)
     */
    public CompletableFuture<String> getCandles(String symbol, String timeframe) ;
    
    /**
     * Get available products list (async)
     *
     * typ: "F" for futures, "O" for options
     */
    public CompletableFuture<String> getProducts(String typ) ;
    
    /**
     * Get quote for a futures/options contract (async)
     *
     * after_hours: true for after-hours session
     */
    public CompletableFuture<String> getQuote(String symbol, Boolean afterHours) ;
    
    /**
     * Get ticker info for a contract (async)
     */
    public CompletableFuture<String> getTicker(String symbol, Boolean afterHours) ;
    
    /**
     * Get batch tickers for futures/options (async)
     *
     * typ: "F" for futures, "O" for options
     */
    public CompletableFuture<String> getTickers(String typ, Boolean isSpread) ;
    
    /**
     * Get trade history for a futures/options contract (async)
     */
    public CompletableFuture<String> getTrades(String symbol) ;
    
    /**
     * Get volume breakdown by price for a futures/options contract (async)
     */
    public CompletableFuture<String> getVolumes(String symbol) ;
    
    /**
     * Get available products list (sync/blocking)
     */
    public String productsSync(String typ) throws MarketDataException;
    
    /**
     * Get quote for a futures/options contract (sync/blocking)
     */
    public String quoteSync(String symbol, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get ticker info for a contract (sync/blocking)
     */
    public String tickerSync(String symbol, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get batch tickers for futures/options (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    public String tickersSync(String typ, Boolean isSpread) throws MarketDataException;
    
    /**
     * Get trade history for a contract (sync/blocking)
     */
    public String tradesSync(String symbol) throws MarketDataException;
    
    /**
     * Get volume breakdown by price for a contract (sync/blocking)
     */
    public String volumesSync(String symbol) throws MarketDataException;
    
}

