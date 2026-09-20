package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for the three `stock/corporate-actions` endpoints.
 *
 * `capital-changes` has no `exchange`: setting it there is 1005
 * `INVALID_PARAMETER`.
 */
public class CorporateActionsParams {
    /**
     * `YYYY-MM-DD`.
     */
    private String startDate;
    /**
     * `YYYY-MM-DD`.
     */
    private String endDate;
    /**
     * `TWSE` or `TPEx` (dividends and listing-applicants only).
     */
    private String exchange;
    /**
     * `asc` or `desc`.
     */
    private String sort;

    public CorporateActionsParams(
        String startDate, 
        String endDate, 
        String exchange, 
        String sort
    ) {
        
        this.startDate = startDate;
        
        this.endDate = endDate;
        
        this.exchange = exchange;
        
        this.sort = sort;
    }
    
    public String startDate() {
        return this.startDate;
    }
    
    public String endDate() {
        return this.endDate;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public String sort() {
        return this.sort;
    }
    public void setStartDate(String startDate) {
        this.startDate = startDate;
    }
    public void setEndDate(String endDate) {
        this.endDate = endDate;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setSort(String sort) {
        this.sort = sort;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof CorporateActionsParams) {
            CorporateActionsParams t = (CorporateActionsParams) other;
            return (
              Objects.equals(startDate, t.startDate) && 
              
              Objects.equals(endDate, t.endDate) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(sort, t.sort)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(startDate, endDate, exchange, sort);
    }
}


