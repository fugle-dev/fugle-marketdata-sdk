package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * RSI data point
 */
public class RsiDataPoint {
    private String date;
    private Double rsi;

    public RsiDataPoint(
        String date, 
        Double rsi
    ) {
        
        this.date = date;
        
        this.rsi = rsi;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double rsi() {
        return this.rsi;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setRsi(Double rsi) {
        this.rsi = rsi;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof RsiDataPoint) {
            RsiDataPoint t = (RsiDataPoint) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(rsi, t.rsi)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, rsi);
    }
}


