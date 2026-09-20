package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * FutOpt intraday endpoints
 */
public interface FutOptIntradayClientInterface {
    
    /**
     * Get candlestick data for a contract (sync/blocking)
     */
    public String candlesSync(String symbol, FutOptCandlesParams params) throws MarketDataException;
    
    /**
     * Get candlestick data for a futures/options contract (async)
     */
    public CompletableFuture<String> getCandles(String symbol, FutOptCandlesParams params) ;
    
    /**
     * Get available products list (async)
     *
     * typ: "F" for futures, "O" for options
     */
    public CompletableFuture<String> getProducts(String typ, FutOptProductsParams params) ;
    
    /**
     * Get quote for a futures/options contract (async)
     */
    public CompletableFuture<String> getQuote(String symbol, AfterHoursParams params) ;
    
    /**
     * Get ticker info for a contract (async)
     */
    public CompletableFuture<String> getTicker(String symbol, AfterHoursParams params) ;
    
    /**
     * Get batch tickers for futures/options (async)
     *
     * typ: "F" for futures, "O" for options
     */
    public CompletableFuture<String> getTickers(String typ, FutOptTickersParams params) ;
    
    /**
     * Get trade history for a futures/options contract (async)
     */
    public CompletableFuture<String> getTrades(String symbol, FutOptTradesParams params) ;
    
    /**
     * Get volume breakdown by price for a futures/options contract (async)
     */
    public CompletableFuture<String> getVolumes(String symbol, AfterHoursParams params) ;
    
    /**
     * Get available products list (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    public String productsSync(String typ, FutOptProductsParams params) throws MarketDataException;
    
    /**
     * Get quote for a futures/options contract (sync/blocking)
     */
    public String quoteSync(String symbol, AfterHoursParams params) throws MarketDataException;
    
    /**
     * Get ticker info for a contract (sync/blocking)
     */
    public String tickerSync(String symbol, AfterHoursParams params) throws MarketDataException;
    
    /**
     * Get batch tickers for futures/options (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    public String tickersSync(String typ, FutOptTickersParams params) throws MarketDataException;
    
    /**
     * Get trade history for a contract (sync/blocking)
     */
    public String tradesSync(String symbol, FutOptTradesParams params) throws MarketDataException;
    
    /**
     * Get volume breakdown by price for a contract (sync/blocking)
     */
    public String volumesSync(String symbol, AfterHoursParams params) throws MarketDataException;
    
}

