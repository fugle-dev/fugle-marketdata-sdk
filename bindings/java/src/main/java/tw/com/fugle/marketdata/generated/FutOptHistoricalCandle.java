package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt historical candle
 */
public class FutOptHistoricalCandle {
    private String date;
    private Double open;
    private Double high;
    private Double low;
    private Double close;
    private Long volume;
    private Long openInterest;
    private Double change;
    private Double changePercent;

    public FutOptHistoricalCandle(
        String date, 
        Double open, 
        Double high, 
        Double low, 
        Double close, 
        Long volume, 
        Long openInterest, 
        Double change, 
        Double changePercent
    ) {
        
        this.date = date;
        
        this.open = open;
        
        this.high = high;
        
        this.low = low;
        
        this.close = close;
        
        this.volume = volume;
        
        this.openInterest = openInterest;
        
        this.change = change;
        
        this.changePercent = changePercent;
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
    
    public Long openInterest() {
        return this.openInterest;
    }
    
    public Double change() {
        return this.change;
    }
    
    public Double changePercent() {
        return this.changePercent;
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
    public void setOpenInterest(Long openInterest) {
        this.openInterest = openInterest;
    }
    public void setChange(Double change) {
        this.change = change;
    }
    public void setChangePercent(Double changePercent) {
        this.changePercent = changePercent;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptHistoricalCandle) {
            FutOptHistoricalCandle t = (FutOptHistoricalCandle) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(open, t.open) && 
              
              Objects.equals(high, t.high) && 
              
              Objects.equals(low, t.low) && 
              
              Objects.equals(close, t.close) && 
              
              Objects.equals(volume, t.volume) && 
              
              Objects.equals(openInterest, t.openInterest) && 
              
              Objects.equals(change, t.change) && 
              
              Objects.equals(changePercent, t.changePercent)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, open, high, low, close, volume, openInterest, change, changePercent);
    }
}


