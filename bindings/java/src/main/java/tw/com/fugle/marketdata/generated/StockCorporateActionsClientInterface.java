package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Stock corporate actions endpoints
 *
 * Provides access to capital changes, dividends, and listing applicants (IPO).
 */
public interface StockCorporateActionsClientInterface {
    
    /**
     * Get capital structure changes (sync/blocking)
     */
    public String capitalChangesSync(String startDate, String endDate) throws MarketDataException;
    
    /**
     * Get dividend announcements (sync/blocking)
     */
    public String dividendsSync(String startDate, String endDate) throws MarketDataException;
    
    /**
     * Get capital structure changes (async)
     */
    public CompletableFuture<String> getCapitalChanges(String startDate, String endDate) ;
    
    /**
     * Get dividend announcements (async)
     */
    public CompletableFuture<String> getDividends(String startDate, String endDate) ;
    
    /**
     * Get IPO listing applicants (async)
     */
    public CompletableFuture<String> getListingApplicants(String startDate, String endDate) ;
    
    /**
     * Get IPO listing applicants (sync/blocking)
     */
    public String listingApplicantsSync(String startDate, String endDate) throws MarketDataException;
    
}

