package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Bollinger Bands data point
 */
public class BbDataPoint {
    private String date;
    private Double upper;
    private Double middle;
    private Double lower;

    public BbDataPoint(
        String date, 
        Double upper, 
        Double middle, 
        Double lower
    ) {
        
        this.date = date;
        
        this.upper = upper;
        
        this.middle = middle;
        
        this.lower = lower;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double upper() {
        return this.upper;
    }
    
    public Double middle() {
        return this.middle;
    }
    
    public Double lower() {
        return this.lower;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setUpper(Double upper) {
        this.upper = upper;
    }
    public void setMiddle(Double middle) {
        this.middle = middle;
    }
    public void setLower(Double lower) {
        this.lower = lower;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof BbDataPoint) {
            BbDataPoint t = (BbDataPoint) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(upper, t.upper) && 
              
              Objects.equals(middle, t.middle) && 
              
              Objects.equals(lower, t.lower)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, upper, middle, lower);
    }
}


