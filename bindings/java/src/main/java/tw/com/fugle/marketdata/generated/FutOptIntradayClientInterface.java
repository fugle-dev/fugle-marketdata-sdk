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
    public IntradayCandlesResponse candlesSync(String symbol, String timeframe) throws MarketDataException;
    
    /**
     * Get candlestick data for a futures/options contract (async)
     */
    public CompletableFuture<IntradayCandlesResponse> getCandles(String symbol, String timeframe) ;
    
    /**
     * Get available products list (async)
     *
     * typ: "F" for futures, "O" for options
     */
    public CompletableFuture<ProductsResponse> getProducts(String typ) ;
    
    /**
     * Get quote for a futures/options contract (async)
     *
     * after_hours: true for after-hours session
     */
    public CompletableFuture<FutOptQuote> getQuote(String symbol, Boolean afterHours) ;
    
    /**
     * Get ticker info for a contract (async)
     */
    public CompletableFuture<FutOptTicker> getTicker(String symbol, Boolean afterHours) ;
    
    /**
     * Get batch tickers for futures/options (async)
     *
     * typ: "F" for futures, "O" for options
     */
    public CompletableFuture<List<FutOptTicker>> getTickers(String typ) ;
    
    /**
     * Get trade history for a futures/options contract (async)
     */
    public CompletableFuture<TradesResponse> getTrades(String symbol) ;
    
    /**
     * Get volume breakdown by price for a futures/options contract (async)
     */
    public CompletableFuture<VolumesResponse> getVolumes(String symbol) ;
    
    /**
     * Get available products list (sync/blocking)
     */
    public ProductsResponse productsSync(String typ) throws MarketDataException;
    
    /**
     * Get quote for a futures/options contract (sync/blocking)
     */
    public FutOptQuote quoteSync(String symbol, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get ticker info for a contract (sync/blocking)
     */
    public FutOptTicker tickerSync(String symbol, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get batch tickers for futures/options (sync/blocking)
     *
     * typ: "F" for futures, "O" for options
     */
    public List<FutOptTicker> tickersSync(String typ) throws MarketDataException;
    
    /**
     * Get trade history for a contract (sync/blocking)
     */
    public TradesResponse tradesSync(String symbol) throws MarketDataException;
    
    /**
     * Get volume breakdown by price for a contract (sync/blocking)
     */
    public VolumesResponse volumesSync(String symbol) throws MarketDataException;
    
}

