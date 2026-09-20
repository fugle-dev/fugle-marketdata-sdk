package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Filters for `futopt/intraday/tickers`; `type` is the method's argument.
 */
public class FutOptTickersParams {
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
     * Product code, `TXF`.
     */
    private String product;
    /**
     * `I`, `R`, `B`, `C`, `S` or `E`.
     */
    private String contractType;
    private Boolean isSpread;

    public FutOptTickersParams(
        String exchange, 
        Boolean afterHours, 
        String product, 
        String contractType, 
        Boolean isSpread
    ) {
        
        this.exchange = exchange;
        
        this.afterHours = afterHours;
        
        this.product = product;
        
        this.contractType = contractType;
        
        this.isSpread = isSpread;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    
    public String product() {
        return this.product;
    }
    
    public String contractType() {
        return this.contractType;
    }
    
    public Boolean isSpread() {
        return this.isSpread;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }
    public void setProduct(String product) {
        this.product = product;
    }
    public void setContractType(String contractType) {
        this.contractType = contractType;
    }
    public void setIsSpread(Boolean isSpread) {
        this.isSpread = isSpread;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptTickersParams) {
            FutOptTickersParams t = (FutOptTickersParams) other;
            return (
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(afterHours, t.afterHours) && 
              
              Objects.equals(product, t.product) && 
              
              Objects.equals(contractType, t.contractType) && 
              
              Objects.equals(isSpread, t.isSpread)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(exchange, afterHours, product, contractType, isSpread);
    }
}


