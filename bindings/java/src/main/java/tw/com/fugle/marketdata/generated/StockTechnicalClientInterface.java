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
 * The periods are required by the server and so are positional; the date
 * range is the record.
 */
public interface StockTechnicalClientInterface {
    
    /**
     * Get Bollinger Bands (sync/blocking)
     */
    public String bbSync(String symbol, Integer period, TechnicalParams params) throws MarketDataException;
    
    /**
     * Get Bollinger Bands (async)
     */
    public CompletableFuture<String> getBb(String symbol, Integer period, TechnicalParams params) ;
    
    /**
     * Get KDJ (Stochastic Oscillator) (async)
     */
    public CompletableFuture<String> getKdj(String symbol, Integer rPeriod, Integer kPeriod, Integer dPeriod, TechnicalParams params) ;
    
    /**
     * Get MACD indicator (async)
     */
    public CompletableFuture<String> getMacd(String symbol, Integer fast, Integer slow, Integer signal, TechnicalParams params) ;
    
    /**
     * Get Relative Strength Index (async)
     */
    public CompletableFuture<String> getRsi(String symbol, Integer period, TechnicalParams params) ;
    
    /**
     * Get Simple Moving Average (async)
     */
    public CompletableFuture<String> getSma(String symbol, Integer period, TechnicalParams params) ;
    
    /**
     * Get KDJ (sync/blocking)
     */
    public String kdjSync(String symbol, Integer rPeriod, Integer kPeriod, Integer dPeriod, TechnicalParams params) throws MarketDataException;
    
    /**
     * Get MACD (sync/blocking)
     */
    public String macdSync(String symbol, Integer fast, Integer slow, Integer signal, TechnicalParams params) throws MarketDataException;
    
    /**
     * Get Relative Strength Index (sync/blocking)
     */
    public String rsiSync(String symbol, Integer period, TechnicalParams params) throws MarketDataException;
    
    /**
     * Get Simple Moving Average (sync/blocking)
     */
    public String smaSync(String symbol, Integer period, TechnicalParams params) throws MarketDataException;
    
}

