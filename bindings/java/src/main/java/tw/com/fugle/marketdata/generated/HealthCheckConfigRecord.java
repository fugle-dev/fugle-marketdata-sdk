package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Health check configuration record for FFI
 *
 * Every field's zero value means "use default", so a zero-initialized
 * record (C++ `HealthCheckConfigRecord{}`, a Go `HealthCheckConfigRecord{}`
 * literal) is the full default: detection on, no probe, 35 s timeout
 * (#158, #161).
 */
public class HealthCheckConfigRecord {
    /**
     * Whether liveness detection is active; `false` turns it off. Unset
     * (the zero value) takes the core default, which is on.
     */
    private Boolean enabled;
    /**
     * Maximum allowed gap between inbound frames before declaring the
     * connection dead, in milliseconds. Default 35000; floor 5000.
     * Pass 0 to use the default. Does not apply when `probe_enabled` is
     * true.
     */
    private Long heartbeatTimeoutMs;
    /**
     * Confirm a silent connection with a ping before declaring it dead
     * (default: false). After `idle_probe_after_ms` of silence one ping is
     * sent; if nothing arrives within `probe_timeout_ms` the connection is
     * declared dead.
     */
    private Boolean probeEnabled;
    /**
     * Silence before the probe, in milliseconds. Default 30000 (the
     * server's heartbeat period); floor 5000. Pass 0 to use the default.
     */
    private Long idleProbeAfterMs;
    /**
     * Wait for any inbound frame after the probe, in milliseconds.
     * Default 5000; floor 1000. Pass 0 to use the default.
     */
    private Long probeTimeoutMs;

    public HealthCheckConfigRecord(
        Boolean enabled, 
        Long heartbeatTimeoutMs, 
        Boolean probeEnabled, 
        Long idleProbeAfterMs, 
        Long probeTimeoutMs
    ) {
        
        this.enabled = enabled;
        
        this.heartbeatTimeoutMs = heartbeatTimeoutMs;
        
        this.probeEnabled = probeEnabled;
        
        this.idleProbeAfterMs = idleProbeAfterMs;
        
        this.probeTimeoutMs = probeTimeoutMs;
    }
    
    public Boolean enabled() {
        return this.enabled;
    }
    
    public Long heartbeatTimeoutMs() {
        return this.heartbeatTimeoutMs;
    }
    
    public Boolean probeEnabled() {
        return this.probeEnabled;
    }
    
    public Long idleProbeAfterMs() {
        return this.idleProbeAfterMs;
    }
    
    public Long probeTimeoutMs() {
        return this.probeTimeoutMs;
    }
    public void setEnabled(Boolean enabled) {
        this.enabled = enabled;
    }
    public void setHeartbeatTimeoutMs(Long heartbeatTimeoutMs) {
        this.heartbeatTimeoutMs = heartbeatTimeoutMs;
    }
    public void setProbeEnabled(Boolean probeEnabled) {
        this.probeEnabled = probeEnabled;
    }
    public void setIdleProbeAfterMs(Long idleProbeAfterMs) {
        this.idleProbeAfterMs = idleProbeAfterMs;
    }
    public void setProbeTimeoutMs(Long probeTimeoutMs) {
        this.probeTimeoutMs = probeTimeoutMs;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof HealthCheckConfigRecord) {
            HealthCheckConfigRecord t = (HealthCheckConfigRecord) other;
            return (
              Objects.equals(enabled, t.enabled) && 
              
              Objects.equals(heartbeatTimeoutMs, t.heartbeatTimeoutMs) && 
              
              Objects.equals(probeEnabled, t.probeEnabled) && 
              
              Objects.equals(idleProbeAfterMs, t.idleProbeAfterMs) && 
              
              Objects.equals(probeTimeoutMs, t.probeTimeoutMs)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(enabled, heartbeatTimeoutMs, probeEnabled, idleProbeAfterMs, probeTimeoutMs);
    }
}


