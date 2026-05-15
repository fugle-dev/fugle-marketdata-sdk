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
    public CapitalChangesResponse capitalChangesSync(String date, String startDate, String endDate) throws MarketDataException;
    
    /**
     * Get dividend announcements (sync/blocking)
     */
    public DividendsResponse dividendsSync(String date, String startDate, String endDate) throws MarketDataException;
    
    /**
     * Get capital structure changes (async)
     */
    public CompletableFuture<CapitalChangesResponse> getCapitalChanges(String date, String startDate, String endDate) ;
    
    /**
     * Get dividend announcements (async)
     */
    public CompletableFuture<DividendsResponse> getDividends(String date, String startDate, String endDate) ;
    
    /**
     * Get IPO listing applicants (async)
     */
    public CompletableFuture<ListingApplicantsResponse> getListingApplicants(String date, String startDate, String endDate) ;
    
    /**
     * Get IPO listing applicants (sync/blocking)
     */
    public ListingApplicantsResponse listingApplicantsSync(String date, String startDate, String endDate) throws MarketDataException;
    
}

