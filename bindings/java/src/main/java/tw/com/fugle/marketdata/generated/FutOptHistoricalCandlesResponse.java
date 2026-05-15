package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt historical candles response
 */
public class FutOptHistoricalCandlesResponse {
    private String symbol;
    private String dataType;
    private String exchange;
    private String timeframe;
    private List<FutOptHistoricalCandle> candles;

    public FutOptHistoricalCandlesResponse(
        String symbol, 
        String dataType, 
        String exchange, 
        String timeframe, 
        List<FutOptHistoricalCandle> candles
    ) {
        
        this.symbol = symbol;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.timeframe = timeframe;
        
        this.candles = candles;
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
    
    public String timeframe() {
        return this.timeframe;
    }
    
    public List<FutOptHistoricalCandle> candles() {
        return this.candles;
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
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }
    public void setCandles(List<FutOptHistoricalCandle> candles) {
        this.candles = candles;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptHistoricalCandlesResponse) {
            FutOptHistoricalCandlesResponse t = (FutOptHistoricalCandlesResponse) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(candles, t.candles)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, dataType, exchange, timeframe, candles);
    }
}


