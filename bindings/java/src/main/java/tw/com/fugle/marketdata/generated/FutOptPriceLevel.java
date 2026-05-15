package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt price level
 */
public class FutOptPriceLevel {
    private Double price;
    private Long size;

    public FutOptPriceLevel(
        Double price, 
        Long size
    ) {
        
        this.price = price;
        
        this.size = size;
    }
    
    public Double price() {
        return this.price;
    }
    
    public Long size() {
        return this.size;
    }
    public void setPrice(Double price) {
        this.price = price;
    }
    public void setSize(Long size) {
        this.size = size;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptPriceLevel) {
            FutOptPriceLevel t = (FutOptPriceLevel) other;
            return (
              Objects.equals(price, t.price) && 
              
              Objects.equals(size, t.size)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(price, size);
    }
}


