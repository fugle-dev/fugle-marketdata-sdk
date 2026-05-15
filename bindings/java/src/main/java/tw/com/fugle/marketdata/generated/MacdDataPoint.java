package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * MACD data point
 */
public class MacdDataPoint {
    private String date;
    private Double macd;
    private Double signalValue;
    private Double histogram;

    public MacdDataPoint(
        String date, 
        Double macd, 
        Double signalValue, 
        Double histogram
    ) {
        
        this.date = date;
        
        this.macd = macd;
        
        this.signalValue = signalValue;
        
        this.histogram = histogram;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double macd() {
        return this.macd;
    }
    
    public Double signalValue() {
        return this.signalValue;
    }
    
    public Double histogram() {
        return this.histogram;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setMacd(Double macd) {
        this.macd = macd;
    }
    public void setSignalValue(Double signalValue) {
        this.signalValue = signalValue;
    }
    public void setHistogram(Double histogram) {
        this.histogram = histogram;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof MacdDataPoint) {
            MacdDataPoint t = (MacdDataPoint) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(macd, t.macd) && 
              
              Objects.equals(signalValue, t.signalValue) && 
              
              Objects.equals(histogram, t.histogram)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, macd, signalValue, histogram);
    }
}


