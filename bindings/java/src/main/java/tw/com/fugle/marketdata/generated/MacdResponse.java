package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * MACD response
 */
public class MacdResponse {
    private String symbol;
    private String dataType;
    private String exchange;
    private String market;
    private String timeframe;
    private Integer fast;
    private Integer slow;
    private Integer signal;
    private List<MacdDataPoint> data;

    public MacdResponse(
        String symbol, 
        String dataType, 
        String exchange, 
        String market, 
        String timeframe, 
        Integer fast, 
        Integer slow, 
        Integer signal, 
        List<MacdDataPoint> data
    ) {
        
        this.symbol = symbol;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.timeframe = timeframe;
        
        this.fast = fast;
        
        this.slow = slow;
        
        this.signal = signal;
        
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
    
    public Integer fast() {
        return this.fast;
    }
    
    public Integer slow() {
        return this.slow;
    }
    
    public Integer signal() {
        return this.signal;
    }
    
    public List<MacdDataPoint> data() {
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
    public void setFast(Integer fast) {
        this.fast = fast;
    }
    public void setSlow(Integer slow) {
        this.slow = slow;
    }
    public void setSignal(Integer signal) {
        this.signal = signal;
    }
    public void setData(List<MacdDataPoint> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof MacdResponse) {
            MacdResponse t = (MacdResponse) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(fast, t.fast) && 
              
              Objects.equals(slow, t.slow) && 
              
              Objects.equals(signal, t.signal) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, dataType, exchange, market, timeframe, fast, slow, signal, data);
    }
}


