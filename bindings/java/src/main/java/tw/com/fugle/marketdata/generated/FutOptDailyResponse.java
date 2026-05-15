package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt daily response
 */
public class FutOptDailyResponse {
    private String symbol;
    private String dataType;
    private String exchange;
    private List<FutOptDailyData> data;

    public FutOptDailyResponse(
        String symbol, 
        String dataType, 
        String exchange, 
        List<FutOptDailyData> data
    ) {
        
        this.symbol = symbol;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.data = data;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String dataType() {
        return this.dataType;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public List<FutOptDailyData> data() {
        return this.data;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setDataType(String dataType) {
        this.dataType = dataType;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setData(List<FutOptDailyData> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptDailyResponse) {
            FutOptDailyResponse t = (FutOptDailyResponse) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, dataType, exchange, data);
    }
}


