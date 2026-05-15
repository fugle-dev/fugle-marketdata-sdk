package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt last trade info
 */
public class FutOptLastTrade {
    private Double price;
    private Long size;
    private Long time;

    public FutOptLastTrade(
        Double price, 
        Long size, 
        Long time
    ) {
        
        this.price = price;
        
        this.size = size;
        
        this.time = time;
    }
    
    public Double price() {
        return this.price;
    }
    
    public Long size() {
        return this.size;
    }
    
    public Long time() {
        return this.time;
    }
    public void setPrice(Double price) {
        this.price = price;
    }
    public void setSize(Long size) {
        this.size = size;
    }
    public void setTime(Long time) {
        this.time = time;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptLastTrade) {
            FutOptLastTrade t = (FutOptLastTrade) other;
            return (
              Objects.equals(price, t.price) && 
              
              Objects.equals(size, t.size) && 
              
              Objects.equals(time, t.time)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(price, size, time);
    }
}


