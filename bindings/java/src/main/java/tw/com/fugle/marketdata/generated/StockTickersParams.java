package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Filters for `stock/intraday/tickers`; `type` is the method's argument.
 */
public class StockTickersParams {
    /**
     * `TWSE` or `TPEx`.
     */
    private String exchange;
    /**
     * `TSE`, `OTC`, `ESB`, `TIB` or `PSB`.
     */
    private String market;
    /**
     * Industry code.
     */
    private String industry;
    private Boolean isNormal;
    private Boolean isAttention;
    private Boolean isDisposition;
    private Boolean isHalted;
    /**
     * Symbol prefix.
     */
    private String symbol;

    public StockTickersParams(
        String exchange, 
        String market, 
        String industry, 
        Boolean isNormal, 
        Boolean isAttention, 
        Boolean isDisposition, 
        Boolean isHalted, 
        String symbol
    ) {
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.industry = industry;
        
        this.isNormal = isNormal;
        
        this.isAttention = isAttention;
        
        this.isDisposition = isDisposition;
        
        this.isHalted = isHalted;
        
        this.symbol = symbol;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public String market() {
        return this.market;
    }
    
    public String industry() {
        return this.industry;
    }
    
    public Boolean isNormal() {
        return this.isNormal;
    }
    
    public Boolean isAttention() {
        return this.isAttention;
    }
    
    public Boolean isDisposition() {
        return this.isDisposition;
    }
    
    public Boolean isHalted() {
        return this.isHalted;
    }
    
    public String symbol() {
        return this.symbol;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setMarket(String market) {
        this.market = market;
    }
    public void setIndustry(String industry) {
        this.industry = industry;
    }
    public void setIsNormal(Boolean isNormal) {
        this.isNormal = isNormal;
    }
    public void setIsAttention(Boolean isAttention) {
        this.isAttention = isAttention;
    }
    public void setIsDisposition(Boolean isDisposition) {
        this.isDisposition = isDisposition;
    }
    public void setIsHalted(Boolean isHalted) {
        this.isHalted = isHalted;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StockTickersParams) {
            StockTickersParams t = (StockTickersParams) other;
            return (
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(industry, t.industry) && 
              
              Objects.equals(isNormal, t.isNormal) && 
              
              Objects.equals(isAttention, t.isAttention) && 
              
              Objects.equals(isDisposition, t.isDisposition) && 
              
              Objects.equals(isHalted, t.isHalted) && 
              
              Objects.equals(symbol, t.symbol)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(exchange, market, industry, isNormal, isAttention, isDisposition, isHalted, symbol);
    }
}


