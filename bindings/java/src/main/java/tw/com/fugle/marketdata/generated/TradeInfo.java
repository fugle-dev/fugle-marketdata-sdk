package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Trade execution info
 */
public class TradeInfo {
    private Double bid;
    private Double ask;
    private Double price;
    private Long size;
    private Long time;

    public TradeInfo(
        Double bid, 
        Double ask, 
        Double price, 
        Long size, 
        Long time
    ) {
        
        this.bid = bid;
        
        this.ask = ask;
        
        this.price = price;
        
        this.size = size;
        
        this.time = time;
    }
    
    public Double bid() {
        return this.bid;
    }
    
    public Double ask() {
        return this.ask;
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
    public void setBid(Double bid) {
        this.bid = bid;
    }
    public void setAsk(Double ask) {
        this.ask = ask;
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
        if (other instanceof TradeInfo) {
            TradeInfo t = (TradeInfo) other;
            return (
              Objects.equals(bid, t.bid) && 
              
              Objects.equals(ask, t.ask) && 
              
              Objects.equals(price, t.price) && 
              
              Objects.equals(size, t.size) && 
              
              Objects.equals(time, t.time)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(bid, ask, price, size, time);
    }
}


