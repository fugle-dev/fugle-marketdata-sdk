package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock technical indicator endpoints
 *
 * Provides access to SMA, RSI, KDJ, MACD, and Bollinger Bands indicators.
 */
public interface StockTechnicalClientInterface {
    
    /**
     * Get Bollinger Bands (sync/blocking)
     */
    public String bbSync(String symbol, String from, String to, String timeframe, Integer period, Double stddev) throws MarketDataException;
    
    /**
     * Get Bollinger Bands (async)
     */
    public CompletableFuture<String> getBb(String symbol, String from, String to, String timeframe, Integer period, Double stddev) ;
    
    /**
     * Get KDJ (Stochastic Oscillator) (async)
     */
    public CompletableFuture<String> getKdj(String symbol, String from, String to, String timeframe, Integer rPeriod, Integer kPeriod, Integer dPeriod) ;
    
    /**
     * Get MACD indicator (async)
     */
    public CompletableFuture<String> getMacd(String symbol, String from, String to, String timeframe, Integer fast, Integer slow, Integer signal) ;
    
    /**
     * Get Relative Strength Index (async)
     */
    public CompletableFuture<String> getRsi(String symbol, String from, String to, String timeframe, Integer period) ;
    
    /**
     * Get Simple Moving Average (async)
     */
    public CompletableFuture<String> getSma(String symbol, String from, String to, String timeframe, Integer period) ;
    
    /**
     * Get KDJ (sync/blocking)
     */
    public String kdjSync(String symbol, String from, String to, String timeframe, Integer rPeriod, Integer kPeriod, Integer dPeriod) throws MarketDataException;
    
    /**
     * Get MACD (sync/blocking)
     */
    public String macdSync(String symbol, String from, String to, String timeframe, Integer fast, Integer slow, Integer signal) throws MarketDataException;
    
    /**
     * Get Relative Strength Index (sync/blocking)
     */
    public String rsiSync(String symbol, String from, String to, String timeframe, Integer period) throws MarketDataException;
    
    /**
     * Get Simple Moving Average (sync/blocking)
     */
    public String smaSync(String symbol, String from, String to, String timeframe, Integer period) throws MarketDataException;
    
}

