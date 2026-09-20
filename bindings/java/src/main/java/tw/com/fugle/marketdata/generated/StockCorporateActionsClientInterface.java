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
 * One record serves all three; `capital-changes` has no `exchange`, so
 * setting it there is 1005 `INVALID_PARAMETER`.
 */
public interface StockCorporateActionsClientInterface {
    
    /**
     * Get capital structure changes (sync/blocking)
     */
    public String capitalChangesSync(CorporateActionsParams params) throws MarketDataException;
    
    /**
     * Get dividend announcements (sync/blocking)
     */
    public String dividendsSync(CorporateActionsParams params) throws MarketDataException;
    
    /**
     * Get capital structure changes (async)
     */
    public CompletableFuture<String> getCapitalChanges(CorporateActionsParams params) ;
    
    /**
     * Get dividend announcements (async)
     */
    public CompletableFuture<String> getDividends(CorporateActionsParams params) ;
    
    /**
     * Get IPO listing applicants (async)
     */
    public CompletableFuture<String> getListingApplicants(CorporateActionsParams params) ;
    
    /**
     * Get IPO listing applicants (sync/blocking)
     */
    public String listingApplicantsSync(CorporateActionsParams params) throws MarketDataException;
    
}

