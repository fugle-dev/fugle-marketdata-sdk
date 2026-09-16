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
    public String candlesSync(String symbol, String from, String to, String timeframe, Boolean afterHours, String contractMonth, String fields, String sort) throws MarketDataException;
    
    /**
     * Get one trading day's daily quotes for every contract month of a product such as "TXF" (sync/blocking)
     */
    public String dailySync(String symbol, String date, Boolean afterHours) throws MarketDataException;
    
    /**
     * Get historical candles for a product such as "TXF" (async)
     *
     * `contract_month` is "YYYYMM" or a continuous contract ("1!", the server
     * default, "2!", "3!").
     */
    public CompletableFuture<String> getCandles(String symbol, String from, String to, String timeframe, Boolean afterHours, String contractMonth, String fields, String sort) ;
    
    /**
     * Get one trading day's daily quotes for every contract month of a product such as "TXF" (async)
     */
    public CompletableFuture<String> getDaily(String symbol, String date, Boolean afterHours) ;
    
}

