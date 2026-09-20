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
     * Get historical candles for a product such as "TXF" (sync/blocking)
     */
    public String candlesSync(String symbol, FutOptHistoricalCandlesParams params) throws MarketDataException;
    
    /**
     * Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
     */
    public String dailySync(String symbol, FutOptDailyParams params) throws MarketDataException;
    
    /**
     * Get historical candles for a product such as "TXF" (async)
     */
    public CompletableFuture<String> getCandles(String symbol, FutOptHistoricalCandlesParams params) ;
    
    /**
     * Get one trading day's daily quotes for every contract month of a product such as "TXF" (async)
     */
    public CompletableFuture<String> getDaily(String symbol, FutOptDailyParams params) ;
    
}

