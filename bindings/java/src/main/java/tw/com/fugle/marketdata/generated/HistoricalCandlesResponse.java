package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Historical candles response
 */
public class HistoricalCandlesResponse {
    private String symbol;
    private String dataType;
    private String exchange;
    private String market;
    private String timeframe;
    private Boolean adjusted;
    private List<HistoricalCandle> data;

    public HistoricalCandlesResponse(
        String symbol, 
        String dataType, 
        String exchange, 
        String market, 
        String timeframe, 
        Boolean adjusted, 
        List<HistoricalCandle> data
    ) {
        
        this.symbol = symbol;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.timeframe = timeframe;
        
        this.adjusted = adjusted;
        
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
    
    public Boolean adjusted() {
        return this.adjusted;
    }
    
    public List<HistoricalCandle> data() {
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
    public void setAdjusted(Boolean adjusted) {
        this.adjusted = adjusted;
    }
    public void setData(List<HistoricalCandle> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof HistoricalCandlesResponse) {
            HistoricalCandlesResponse t = (HistoricalCandlesResponse) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(adjusted, t.adjusted) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, dataType, exchange, market, timeframe, adjusted, data);
    }
}


