package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Volumes response
 */
public class VolumesResponse {
    private String date;
    private String dataType;
    private String exchange;
    private String market;
    private String symbol;
    private List<VolumeAtPrice> data;

    public VolumesResponse(
        String date, 
        String dataType, 
        String exchange, 
        String market, 
        String symbol, 
        List<VolumeAtPrice> data
    ) {
        
        this.date = date;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.symbol = symbol;
        
        this.data = data;
    }
    
    public String date() {
        return this.date;
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
    
    public List<VolumeAtPrice> data() {
        return this.data;
    }
    public void setDate(String date) {
        this.date = date;
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
    public void setData(List<VolumeAtPrice> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof VolumesResponse) {
            VolumesResponse t = (VolumesResponse) other;
            return (
              Objects.equals(date, t.date) && 
              
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
        return Objects.hash(date, dataType, exchange, market, symbol, data);
    }
}


