package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Intraday candles response
 */
public class IntradayCandlesResponse {
    private String date;
    private String dataType;
    private String exchange;
    private String market;
    private String symbol;
    private String timeframe;
    private List<IntradayCandle> data;

    public IntradayCandlesResponse(
        String date, 
        String dataType, 
        String exchange, 
        String market, 
        String symbol, 
        String timeframe, 
        List<IntradayCandle> data
    ) {
        
        this.date = date;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.symbol = symbol;
        
        this.timeframe = timeframe;
        
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
    
    public String timeframe() {
        return this.timeframe;
    }
    
    public List<IntradayCandle> data() {
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
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }
    public void setData(List<IntradayCandle> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof IntradayCandlesResponse) {
            IntradayCandlesResponse t = (IntradayCandlesResponse) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, dataType, exchange, market, symbol, timeframe, data);
    }
}


