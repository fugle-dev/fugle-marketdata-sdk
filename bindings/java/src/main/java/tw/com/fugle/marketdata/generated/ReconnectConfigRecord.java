package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Reconnection configuration record for FFI
 *
 * Without a record the client auto-reconnects with the core defaults. In a
 * record, `enabled` is taken as given and zero numeric fields mean "use
 * default".
 */
public class ReconnectConfigRecord {
    /**
     * Whether auto-reconnect is active; `false` turns it off
     */
    private Boolean enabled;
    /**
     * Maximum reconnection attempts; 0 means unlimited (the default)
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
        Boolean enabled, 
        Integer maxAttempts, 
        Long initialDelayMs, 
        Long maxDelayMs
    ) {
        
        this.enabled = enabled;
        
        this.maxAttempts = maxAttempts;
        
        this.initialDelayMs = initialDelayMs;
        
        this.maxDelayMs = maxDelayMs;
    }
    
    public Boolean enabled() {
        return this.enabled;
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
    public void setEnabled(Boolean enabled) {
        this.enabled = enabled;
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
              Objects.equals(enabled, t.enabled) && 
              
              Objects.equals(maxAttempts, t.maxAttempts) && 
              
              Objects.equals(initialDelayMs, t.initialDelayMs) && 
              
              Objects.equals(maxDelayMs, t.maxDelayMs)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(enabled, maxAttempts, initialDelayMs, maxDelayMs);
    }
}


