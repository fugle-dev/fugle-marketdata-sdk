package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * FutOpt historical data endpoints
 *
 * Provides access to historical candles and daily data for futures and options.
 */
public interface FutOptHistoricalClientInterface {
    
    /**
     * Get historical candles for a contract (sync/blocking)
     */
    public FutOptHistoricalCandlesResponse candlesSync(String symbol, String from, String to, String timeframe, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get daily historical data for a contract (sync/blocking)
     */
    public FutOptDailyResponse dailySync(String symbol, String from, String to, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get historical candles for a contract (async)
     */
    public CompletableFuture<FutOptHistoricalCandlesResponse> getCandles(String symbol, String from, String to, String timeframe, Boolean afterHours) ;
    
    /**
     * Get daily historical data for a contract (async)
     */
    public CompletableFuture<FutOptDailyResponse> getDaily(String symbol, String from, String to, Boolean afterHours) ;
    
}

