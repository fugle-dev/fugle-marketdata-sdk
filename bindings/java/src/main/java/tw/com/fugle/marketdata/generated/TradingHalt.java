package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Trading halt status
 */
public class TradingHalt {
    private Boolean isHalted;
    private Long time;

    public TradingHalt(
        Boolean isHalted, 
        Long time
    ) {
        
        this.isHalted = isHalted;
        
        this.time = time;
    }
    
    public Boolean isHalted() {
        return this.isHalted;
    }
    
    public Long time() {
        return this.time;
    }
    public void setIsHalted(Boolean isHalted) {
        this.isHalted = isHalted;
    }
    public void setTime(Long time) {
        this.time = time;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof TradingHalt) {
            TradingHalt t = (TradingHalt) other;
            return (
              Objects.equals(isHalted, t.isHalted) && 
              
              Objects.equals(time, t.time)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(isHalted, time);
    }
}


