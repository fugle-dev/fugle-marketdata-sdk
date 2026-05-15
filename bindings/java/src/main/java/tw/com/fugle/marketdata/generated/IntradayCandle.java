package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Single intraday candle
 */
public class IntradayCandle {
    private Double open;
    private Double high;
    private Double low;
    private Double close;
    private Long volume;
    private Double average;
    private String date;

    public IntradayCandle(
        Double open, 
        Double high, 
        Double low, 
        Double close, 
        Long volume, 
        Double average, 
        String date
    ) {
        
        this.open = open;
        
        this.high = high;
        
        this.low = low;
        
        this.close = close;
        
        this.volume = volume;
        
        this.average = average;
        
        this.date = date;
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
    
    public Double average() {
        return this.average;
    }
    
    public String date() {
        return this.date;
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
    public void setAverage(Double average) {
        this.average = average;
    }
    public void setDate(String date) {
        this.date = date;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof IntradayCandle) {
            IntradayCandle t = (IntradayCandle) other;
            return (
              Objects.equals(open, t.open) && 
              
              Objects.equals(high, t.high) && 
              
              Objects.equals(low, t.low) && 
              
              Objects.equals(close, t.close) && 
              
              Objects.equals(volume, t.volume) && 
              
              Objects.equals(average, t.average) && 
              
              Objects.equals(date, t.date)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(open, high, low, close, volume, average, date);
    }
}


