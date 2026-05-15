package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * KDJ data point
 */
public class KdjDataPoint {
    private String date;
    private Double k;
    private Double d;
    private Double j;

    public KdjDataPoint(
        String date, 
        Double k, 
        Double d, 
        Double j
    ) {
        
        this.date = date;
        
        this.k = k;
        
        this.d = d;
        
        this.j = j;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double k() {
        return this.k;
    }
    
    public Double d() {
        return this.d;
    }
    
    public Double j() {
        return this.j;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setK(Double k) {
        this.k = k;
    }
    public void setD(Double d) {
        this.d = d;
    }
    public void setJ(Double j) {
        this.j = j;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof KdjDataPoint) {
            KdjDataPoint t = (KdjDataPoint) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(k, t.k) && 
              
              Objects.equals(d, t.d) && 
              
              Objects.equals(j, t.j)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, k, d, j);
    }
}


