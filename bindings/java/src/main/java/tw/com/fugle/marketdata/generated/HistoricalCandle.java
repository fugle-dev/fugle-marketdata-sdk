package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Single historical candle
 */
public class HistoricalCandle {
    private String date;
    private Double open;
    private Double high;
    private Double low;
    private Double close;
    private Long volume;
    private Double turnover;
    private Double change;

    public HistoricalCandle(
        String date, 
        Double open, 
        Double high, 
        Double low, 
        Double close, 
        Long volume, 
        Double turnover, 
        Double change
    ) {
        
        this.date = date;
        
        this.open = open;
        
        this.high = high;
        
        this.low = low;
        
        this.close = close;
        
        this.volume = volume;
        
        this.turnover = turnover;
        
        this.change = change;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double open() {
        return this.open;
    }
    
    public Double high() {
        return this.high;
    }
    
    public Double low() {
        return this.low;
    }
    
    public Double close() {
        return this.close;
    }
    
    public Long volume() {
        return this.volume;
    }
    
    public Double turnover() {
        return this.turnover;
    }
    
    public Double change() {
        return this.change;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setOpen(Double open) {
        this.open = open;
    }
    public void setHigh(Double high) {
        this.high = high;
    }
    public void setLow(Double low) {
        this.low = low;
    }
    public void setClose(Double close) {
        this.close = close;
    }
    public void setVolume(Long volume) {
        this.volume = volume;
    }
    public void setTurnover(Double turnover) {
        this.turnover = turnover;
    }
    public void setChange(Double change) {
        this.change = change;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof HistoricalCandle) {
            HistoricalCandle t = (HistoricalCandle) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(open, t.open) && 
              
              Objects.equals(high, t.high) && 
              
              Objects.equals(low, t.low) && 
              
              Objects.equals(close, t.close) && 
              
              Objects.equals(volume, t.volume) && 
              
              Objects.equals(turnover, t.turnover) && 
              
              Objects.equals(change, t.change)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, open, high, low, close, volume, turnover, change);
    }
}


