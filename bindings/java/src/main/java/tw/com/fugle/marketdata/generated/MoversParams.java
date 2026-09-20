package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `stock/snapshot/movers`; `direction` and `change` are the
 * method's arguments.
 *
 * The price bounds are `f64`, and 0 is a bound like any other: unlike the
 * config records, an unset field is `None`, not 0.
 */
public class MoversParams {
    /**
     * `type`: `ALL`, `ALLBUT0999` or `COMMONSTOCK`.
     */
    private String typeFilter;
    /**
     * Change greater than.
     */
    private Double gt;
    /**
     * Change greater than or equal to.
     */
    private Double gte;
    /**
     * Change less than.
     */
    private Double lt;
    /**
     * Change less than or equal to.
     */
    private Double lte;
    /**
     * Change equal to.
     */
    private Double eq;

    public MoversParams(
        String typeFilter, 
        Double gt, 
        Double gte, 
        Double lt, 
        Double lte, 
        Double eq
    ) {
        
        this.typeFilter = typeFilter;
        
        this.gt = gt;
        
        this.gte = gte;
        
        this.lt = lt;
        
        this.lte = lte;
        
        this.eq = eq;
    }
    
    public String typeFilter() {
        return this.typeFilter;
    }
    
    public Double gt() {
        return this.gt;
    }
    
    public Double gte() {
        return this.gte;
    }
    
    public Double lt() {
        return this.lt;
    }
    
    public Double lte() {
        return this.lte;
    }
    
    public Double eq() {
        return this.eq;
    }
    public void setTypeFilter(String typeFilter) {
        this.typeFilter = typeFilter;
    }
    public void setGt(Double gt) {
        this.gt = gt;
    }
    public void setGte(Double gte) {
        this.gte = gte;
    }
    public void setLt(Double lt) {
        this.lt = lt;
    }
    public void setLte(Double lte) {
        this.lte = lte;
    }
    public void setEq(Double eq) {
        this.eq = eq;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof MoversParams) {
            MoversParams t = (MoversParams) other;
            return (
              Objects.equals(typeFilter, t.typeFilter) && 
              
              Objects.equals(gt, t.gt) && 
              
              Objects.equals(gte, t.gte) && 
              
              Objects.equals(lt, t.lt) && 
              
              Objects.equals(lte, t.lte) && 
              
              Objects.equals(eq, t.eq)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(typeFilter, gt, gte, lt, lte, eq);
    }
}


