package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Response for `stock/ownership/director-holdings/{symbol}`
 */
public class DirectorHoldingsResponse {
    private String dataType;
    private String exchange;
    private String market;
    private String symbol;
    private List<DirectorHoldingsEntry> data;

    public DirectorHoldingsResponse(
        String dataType, 
        String exchange, 
        String market, 
        String symbol, 
        List<DirectorHoldingsEntry> data
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
    
    public List<DirectorHoldingsEntry> data() {
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
    public void setData(List<DirectorHoldingsEntry> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof DirectorHoldingsResponse) {
            DirectorHoldingsResponse t = (DirectorHoldingsResponse) other;
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


