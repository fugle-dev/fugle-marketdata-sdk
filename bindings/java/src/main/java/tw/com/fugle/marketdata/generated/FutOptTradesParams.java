package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `futopt/intraday/trades`.
 */
public class FutOptTradesParams {
    /**
     * `true` asks for the after-hours session (`session=afterhours`).
     */
    private Boolean afterHours;
    private Integer offset;
    private Integer limit;
    private Boolean isTrial;

    public FutOptTradesParams(
        Boolean afterHours, 
        Integer offset, 
        Integer limit, 
        Boolean isTrial
    ) {
        
        this.afterHours = afterHours;
        
        this.offset = offset;
        
        this.limit = limit;
        
        this.isTrial = isTrial;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    
    public Integer offset() {
        return this.offset;
    }
    
    public Integer limit() {
        return this.limit;
    }
    
    public Boolean isTrial() {
        return this.isTrial;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }
    public void setOffset(Integer offset) {
        this.offset = offset;
    }
    public void setLimit(Integer limit) {
        this.limit = limit;
    }
    public void setIsTrial(Boolean isTrial) {
        this.isTrial = isTrial;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptTradesParams) {
            FutOptTradesParams t = (FutOptTradesParams) other;
            return (
              Objects.equals(afterHours, t.afterHours) && 
              
              Objects.equals(offset, t.offset) && 
              
              Objects.equals(limit, t.limit) && 
              
              Objects.equals(isTrial, t.isTrial)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(afterHours, offset, limit, isTrial);
    }
}


