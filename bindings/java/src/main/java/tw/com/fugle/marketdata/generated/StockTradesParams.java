package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `stock/intraday/trades`.
 */
public class StockTradesParams {
    /**
     * `true` asks for the intraday odd-lot session (`type=oddlot`).
     */
    private Boolean oddLot;
    private Integer offset;
    private Integer limit;
    /**
     * `asc` or `desc`.
     */
    private String sort;
    private Boolean isTrial;

    public StockTradesParams(
        Boolean oddLot, 
        Integer offset, 
        Integer limit, 
        String sort, 
        Boolean isTrial
    ) {
        
        this.oddLot = oddLot;
        
        this.offset = offset;
        
        this.limit = limit;
        
        this.sort = sort;
        
        this.isTrial = isTrial;
    }
    
    public Boolean oddLot() {
        return this.oddLot;
    }
    
    public Integer offset() {
        return this.offset;
    }
    
    public Integer limit() {
        return this.limit;
    }
    
    public String sort() {
        return this.sort;
    }
    
    public Boolean isTrial() {
        return this.isTrial;
    }
    public void setOddLot(Boolean oddLot) {
        this.oddLot = oddLot;
    }
    public void setOffset(Integer offset) {
        this.offset = offset;
    }
    public void setLimit(Integer limit) {
        this.limit = limit;
    }
    public void setSort(String sort) {
        this.sort = sort;
    }
    public void setIsTrial(Boolean isTrial) {
        this.isTrial = isTrial;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StockTradesParams) {
            StockTradesParams t = (StockTradesParams) other;
            return (
              Objects.equals(oddLot, t.oddLot) && 
              
              Objects.equals(offset, t.offset) && 
              
              Objects.equals(limit, t.limit) && 
              
              Objects.equals(sort, t.sort) && 
              
              Objects.equals(isTrial, t.isTrial)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(oddLot, offset, limit, sort, isTrial);
    }
}


