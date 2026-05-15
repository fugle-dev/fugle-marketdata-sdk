package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Reconnection configuration record for FFI
 *
 * All fields are optional — zero/false values mean "use default".
 */
public class ReconnectConfigRecord {
    /**
     * Maximum reconnection attempts (default: 5, min: 1)
     */
    private Integer maxAttempts;
    /**
     * Initial reconnection delay in milliseconds (default: 1000, min: 100)
     */
    private Long initialDelayMs;
    /**
     * Maximum reconnection delay in milliseconds (default: 60000)
     */
    private Long maxDelayMs;

    public ReconnectConfigRecord(
        Integer maxAttempts, 
        Long initialDelayMs, 
        Long maxDelayMs
    ) {
        
        this.maxAttempts = maxAttempts;
        
        this.initialDelayMs = initialDelayMs;
        
        this.maxDelayMs = maxDelayMs;
    }
    
    public Integer maxAttempts() {
        return this.maxAttempts;
    }
    
    public Long initialDelayMs() {
        return this.initialDelayMs;
    }
    
    public Long maxDelayMs() {
        return this.maxDelayMs;
    }
    public void setMaxAttempts(Integer maxAttempts) {
        this.maxAttempts = maxAttempts;
    }
    public void setInitialDelayMs(Long initialDelayMs) {
        this.initialDelayMs = initialDelayMs;
    }
    public void setMaxDelayMs(Long maxDelayMs) {
        this.maxDelayMs = maxDelayMs;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof ReconnectConfigRecord) {
            ReconnectConfigRecord t = (ReconnectConfigRecord) other;
            return (
              Objects.equals(maxAttempts, t.maxAttempts) && 
              
              Objects.equals(initialDelayMs, t.initialDelayMs) && 
              
              Objects.equals(maxDelayMs, t.maxDelayMs)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(maxAttempts, initialDelayMs, maxDelayMs);
    }
}


