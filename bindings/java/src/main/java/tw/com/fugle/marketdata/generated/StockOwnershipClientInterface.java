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
     * Get monthly holdings and pledges disclosed by directors and supervisors (async)
     */
    public CompletableFuture<DirectorHoldingsResponse> getDirectorHoldings(String symbol, String from, String to, String sort) ;
    
    /**
     * Get the constituents an ETF held over a date range (async)
     */
    public CompletableFuture<EtfHoldingsResponse> getEtfHoldings(String symbol, String from, String to, String sort) ;
    
    /**
     * Get daily trading by the three major institutional investors (async)
     */
    public CompletableFuture<InstitutionalTradesResponse> getInstitutionalTrades(String symbol, String from, String to, String sort) ;
    
    /**
     * Get the weekly TDCC shareholder distribution by holding-size bracket (async)
     */
    public CompletableFuture<TdccDistributionResponse> getTdccDistribution(String symbol, String from, String to, String sort) ;
    
}

