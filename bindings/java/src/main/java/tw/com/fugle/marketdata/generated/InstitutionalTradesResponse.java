package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Response for `stock/ownership/institutional-trades/{symbol}`
 */
public class InstitutionalTradesResponse {
    private String dataType;
    private String exchange;
    private String market;
    private String symbol;
    private List<InstitutionalTradesEntry> data;

    public InstitutionalTradesResponse(
        String dataType, 
        String exchange, 
        String market, 
        String symbol, 
        List<InstitutionalTradesEntry> data
    ) {
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.symbol = symbol;
        
        this.data = data;
    }
    
    public String dataType() {
        return this.dataType;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public String market() {
        return this.market;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public List<InstitutionalTradesEntry> data() {
        return this.data;
    }
    public void setDataType(String dataType) {
        this.dataType = dataType;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setMarket(String market) {
        this.market = market;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setData(List<InstitutionalTradesEntry> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof InstitutionalTradesResponse) {
            InstitutionalTradesResponse t = (InstitutionalTradesResponse) other;
            return (
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(dataType, exchange, market, symbol, data);
    }
}


