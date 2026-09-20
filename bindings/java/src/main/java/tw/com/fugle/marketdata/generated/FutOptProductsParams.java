package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Filters for `futopt/intraday/products`; `type` is the method's argument.
 */
public class FutOptProductsParams {
    /**
     * `TAIFEX`.
     */
    private String exchange;
    /**
     * `true` asks for the after-hours session (`session=AFTERHOURS`);
     * unset or `false` is the regular session.
     */
    private Boolean afterHours;
    /**
     * `I`, `R`, `B`, `C`, `S` or `E`.
     */
    private String contractType;
    /**
     * `N` (normal) or `U` (unlisted).
     */
    private String status;

    public FutOptProductsParams(
        String exchange, 
        Boolean afterHours, 
        String contractType, 
        String status
    ) {
        
        this.exchange = exchange;
        
        this.afterHours = afterHours;
        
        this.contractType = contractType;
        
        this.status = status;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    
    public String contractType() {
        return this.contractType;
    }
    
    public String status() {
        return this.status;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }
    public void setContractType(String contractType) {
        this.contractType = contractType;
    }
    public void setStatus(String status) {
        this.status = status;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptProductsParams) {
            FutOptProductsParams t = (FutOptProductsParams) other;
            return (
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(afterHours, t.afterHours) && 
              
              Objects.equals(contractType, t.contractType) && 
              
              Objects.equals(status, t.status)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(exchange, afterHours, contractType, status);
    }
}


