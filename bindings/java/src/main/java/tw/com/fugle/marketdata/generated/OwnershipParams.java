package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for the four `stock/ownership` endpoints.
 */
public class OwnershipParams {
    /**
     * Start date, `YYYY-MM-DD`.
     */
    private String from;
    /**
     * End date, `YYYY-MM-DD`.
     */
    private String to;
    /**
     * `asc` or `desc`.
     */
    private String sort;

    public OwnershipParams(
        String from, 
        String to, 
        String sort
    ) {
        
        this.from = from;
        
        this.to = to;
        
        this.sort = sort;
    }
    
    public String from() {
        return this.from;
    }
    
    public String to() {
        return this.to;
    }
    
    public String sort() {
        return this.sort;
    }
    public void setFrom(String from) {
        this.from = from;
    }
    public void setTo(String to) {
        this.to = to;
    }
    public void setSort(String sort) {
        this.sort = sort;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof OwnershipParams) {
            OwnershipParams t = (OwnershipParams) other;
            return (
              Objects.equals(from, t.from) && 
              
              Objects.equals(to, t.to) && 
              
              Objects.equals(sort, t.sort)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(from, to, sort);
    }
}


