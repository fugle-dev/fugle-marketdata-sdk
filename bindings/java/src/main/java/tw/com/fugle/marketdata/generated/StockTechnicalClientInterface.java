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
    public BbResponse bbSync(String symbol, String from, String to, String timeframe, Integer period, Double stddev) throws MarketDataException;
    
    /**
     * Get Bollinger Bands (async)
     */
    public CompletableFuture<BbResponse> getBb(String symbol, String from, String to, String timeframe, Integer period, Double stddev) ;
    
    /**
     * Get KDJ (Stochastic Oscillator) (async)
     */
    public CompletableFuture<KdjResponse> getKdj(String symbol, String from, String to, String timeframe, Integer period) ;
    
    /**
     * Get MACD indicator (async)
     */
    public CompletableFuture<MacdResponse> getMacd(String symbol, String from, String to, String timeframe, Integer fast, Integer slow, Integer signal) ;
    
    /**
     * Get Relative Strength Index (async)
     */
    public CompletableFuture<RsiResponse> getRsi(String symbol, String from, String to, String timeframe, Integer period) ;
    
    /**
     * Get Simple Moving Average (async)
     */
    public CompletableFuture<SmaResponse> getSma(String symbol, String from, String to, String timeframe, Integer period) ;
    
    /**
     * Get KDJ (sync/blocking)
     */
    public KdjResponse kdjSync(String symbol, String from, String to, String timeframe, Integer period) throws MarketDataException;
    
    /**
     * Get MACD (sync/blocking)
     */
    public MacdResponse macdSync(String symbol, String from, String to, String timeframe, Integer fast, Integer slow, Integer signal) throws MarketDataException;
    
    /**
     * Get Relative Strength Index (sync/blocking)
     */
    public RsiResponse rsiSync(String symbol, String from, String to, String timeframe, Integer period) throws MarketDataException;
    
    /**
     * Get Simple Moving Average (sync/blocking)
     */
    public SmaResponse smaSync(String symbol, String from, String to, String timeframe, Integer period) throws MarketDataException;
    
}

