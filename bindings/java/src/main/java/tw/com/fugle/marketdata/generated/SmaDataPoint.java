package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * SMA data point
 */
public class SmaDataPoint {
    private String date;
    private Double sma;

    public SmaDataPoint(
        String date, 
        Double sma
    ) {
        
        this.date = date;
        
        this.sma = sma;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double sma() {
        return this.sma;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setSma(Double sma) {
        this.sma = sma;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof SmaDataPoint) {
            SmaDataPoint t = (SmaDataPoint) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(sma, t.sma)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, sma);
    }
}


