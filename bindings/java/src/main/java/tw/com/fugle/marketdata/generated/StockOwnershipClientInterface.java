package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock ownership endpoints client
 */
public interface StockOwnershipClientInterface {
    
    /**
     * Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking)
     */
    public String directorHoldingsSync(String symbol, String from, String to, String sort) throws MarketDataException;
    
    /**
     * Get the constituents an ETF held over a date range (sync/blocking)
     */
    public String etfHoldingsSync(String symbol, String from, String to, String sort) throws MarketDataException;
    
    /**
     * Get monthly holdings and pledges disclosed by directors and supervisors (async)
     */
    public CompletableFuture<String> getDirectorHoldings(String symbol, String from, String to, String sort) ;
    
    /**
     * Get the constituents an ETF held over a date range (async)
     */
    public CompletableFuture<String> getEtfHoldings(String symbol, String from, String to, String sort) ;
    
    /**
     * Get daily trading by the three major institutional investors (async)
     */
    public CompletableFuture<String> getInstitutionalTrades(String symbol, String from, String to, String sort) ;
    
    /**
     * Get the weekly TDCC shareholder distribution by holding-size bracket (async)
     */
    public CompletableFuture<String> getTdccDistribution(String symbol, String from, String to, String sort) ;
    
    /**
     * Get daily trading by the three major institutional investors (sync/blocking)
     */
    public String institutionalTradesSync(String symbol, String from, String to, String sort) throws MarketDataException;
    
    /**
     * Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking)
     */
    public String tdccDistributionSync(String symbol, String from, String to, String sort) throws MarketDataException;
    
}

