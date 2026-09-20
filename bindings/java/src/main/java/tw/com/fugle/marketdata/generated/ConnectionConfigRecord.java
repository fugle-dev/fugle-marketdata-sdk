package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Connection configuration record for FFI: the timeouts of the connection
 * itself (#199).
 *
 * Every field's zero value means "use default", so a zero-initialized
 * record (C++ `ConnectionConfigRecord{}`, a Go `ConnectionConfigRecord{}`
 * literal) is the full default. Omitting the record gives the same result.
 */
public class ConnectionConfigRecord {
    /**
     * How long the auth handshake may take once the WebSocket is open, in
     * milliseconds: from the auth frame being sent until the server's
     * verdict. Default 10000. Pass 0 to use the default. Applies to the
     * first `connect()` and to every reconnect; elapsing it fails the
     * attempt with a `TimeoutError` (3001). The server itself allows 60 s.
     */
    private Long authTimeoutMs;

    public ConnectionConfigRecord(
        Long authTimeoutMs
    ) {
        
        this.authTimeoutMs = authTimeoutMs;
    }
    
    public Long authTimeoutMs() {
        return this.authTimeoutMs;
    }
    public void setAuthTimeoutMs(Long authTimeoutMs) {
        this.authTimeoutMs = authTimeoutMs;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof ConnectionConfigRecord) {
            ConnectionConfigRecord t = (ConnectionConfigRecord) other;
            return (
              Objects.equals(authTimeoutMs, t.authTimeoutMs)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(authTimeoutMs);
    }
}


