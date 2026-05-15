package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Dividends response
 */
public class DividendsResponse {
    private String dataType;
    private String exchange;
    private String market;
    private List<Dividend> data;

    public DividendsResponse(
        String dataType, 
        String exchange, 
        String market, 
        List<Dividend> data
    ) {
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
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
    
    public List<Dividend> data() {
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
    public void setData(List<Dividend> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof DividendsResponse) {
            DividendsResponse t = (DividendsResponse) other;
            return (
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(dataType, exchange, market, data);
    }
}


