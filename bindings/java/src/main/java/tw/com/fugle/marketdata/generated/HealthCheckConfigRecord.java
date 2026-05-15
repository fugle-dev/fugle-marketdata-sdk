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
     * Whether health check is enabled (default: false)
     */
    private Boolean enabled;
    /**
     * Interval between ping messages in milliseconds (default: 30000, min: 5000)
     */
    private Long intervalMs;
    /**
     * Maximum missed pongs before disconnect (default: 2, min: 1)
     */
    private Long maxMissedPongs;

    public HealthCheckConfigRecord(
        Boolean enabled, 
        Long intervalMs, 
        Long maxMissedPongs
    ) {
        
        this.enabled = enabled;
        
        this.intervalMs = intervalMs;
        
        this.maxMissedPongs = maxMissedPongs;
    }
    
    public Boolean enabled() {
        return this.enabled;
    }
    
    public Long intervalMs() {
        return this.intervalMs;
    }
    
    public Long maxMissedPongs() {
        return this.maxMissedPongs;
    }
    public void setEnabled(Boolean enabled) {
        this.enabled = enabled;
    }
    public void setIntervalMs(Long intervalMs) {
        this.intervalMs = intervalMs;
    }
    public void setMaxMissedPongs(Long maxMissedPongs) {
        this.maxMissedPongs = maxMissedPongs;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof HealthCheckConfigRecord) {
            HealthCheckConfigRecord t = (HealthCheckConfigRecord) other;
            return (
              Objects.equals(enabled, t.enabled) && 
              
              Objects.equals(intervalMs, t.intervalMs) && 
              
              Objects.equals(maxMissedPongs, t.maxMissedPongs)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(enabled, intervalMs, maxMissedPongs);
    }
}


