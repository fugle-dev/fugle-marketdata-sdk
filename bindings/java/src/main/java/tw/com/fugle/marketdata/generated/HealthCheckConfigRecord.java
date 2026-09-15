package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Health check configuration record for FFI
 *
 * All fields are optional — zero/false values mean "use default".
 */
public class HealthCheckConfigRecord {
    /**
     * Whether liveness detection is active (default: true in 3.0)
     */
    private Boolean enabled;
    /**
     * Maximum allowed gap between inbound frames before declaring the
     * connection dead, in milliseconds. Default 35000; floor 5000.
     * Pass 0 to use the default.
     */
    private Long heartbeatTimeoutMs;

    public HealthCheckConfigRecord(
        Boolean enabled, 
        Long heartbeatTimeoutMs
    ) {
        
        this.enabled = enabled;
        
        this.heartbeatTimeoutMs = heartbeatTimeoutMs;
    }
    
    public Boolean enabled() {
        return this.enabled;
    }
    
    public Long heartbeatTimeoutMs() {
        return this.heartbeatTimeoutMs;
    }
    public void setEnabled(Boolean enabled) {
        this.enabled = enabled;
    }
    public void setHeartbeatTimeoutMs(Long heartbeatTimeoutMs) {
        this.heartbeatTimeoutMs = heartbeatTimeoutMs;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof HealthCheckConfigRecord) {
            HealthCheckConfigRecord t = (HealthCheckConfigRecord) other;
            return (
              Objects.equals(enabled, t.enabled) && 
              
              Objects.equals(heartbeatTimeoutMs, t.heartbeatTimeoutMs)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(enabled, heartbeatTimeoutMs);
    }
}


