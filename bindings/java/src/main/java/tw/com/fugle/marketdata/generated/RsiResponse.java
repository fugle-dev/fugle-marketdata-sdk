package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * RSI response
 */
public class RsiResponse {
    private String symbol;
    private String dataType;
    private String exchange;
    private String market;
    private String timeframe;
    private Integer period;
    private List<RsiDataPoint> data;

    public RsiResponse(
        String symbol, 
        String dataType, 
        String exchange, 
        String market, 
        String timeframe, 
        Integer period, 
        List<RsiDataPoint> data
    ) {
        
        this.symbol = symbol;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.timeframe = timeframe;
        
        this.period = period;
        
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
    
    public String market() {
        return this.market;
    }
    
    public String timeframe() {
        return this.timeframe;
    }
    
    public Integer period() {
        return this.period;
    }
    
    public List<RsiDataPoint> data() {
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
    public void setMarket(String market) {
        this.market = market;
    }
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }
    public void setPeriod(Integer period) {
        this.period = period;
    }
    public void setData(List<RsiDataPoint> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof RsiResponse) {
            RsiResponse t = (RsiResponse) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(period, t.period) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, dataType, exchange, market, timeframe, period, data);
    }
}


