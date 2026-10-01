package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * The last disconnect of a client: who closed the connection and whether a
 * reconnect follows. Read it with `WebSocketClient::last_disconnect()`
 * (#293).
 */
public class DisconnectInfo {
    /**
     * WebSocket close code, or none when the connection ended without one
     * (transport error, EOF, heartbeat timeout, or a server Close frame
     * without a code).
     */
    private Short code;
    /**
     * Close reason (may be empty).
     */
    private String reason;
    /**
     * Who closed the connection.
     */
    private DisconnectIntent intent;
    /**
     * The `will_reconnect` of the matching `on_disconnected`: true if a
     * reconnect follows (unless `disconnect()` is called first), false if
     * this connection is over.
     */
    private Boolean willReconnect;

    public DisconnectInfo(
        Short code, 
        String reason, 
        DisconnectIntent intent, 
        Boolean willReconnect
    ) {
        
        this.code = code;
        
        this.reason = reason;
        
        this.intent = intent;
        
        this.willReconnect = willReconnect;
    }
    
    public Short code() {
        return this.code;
    }
    
    public String reason() {
        return this.reason;
    }
    
    public DisconnectIntent intent() {
        return this.intent;
    }
    
    public Boolean willReconnect() {
        return this.willReconnect;
    }
    public void setCode(Short code) {
        this.code = code;
    }
    public void setReason(String reason) {
        this.reason = reason;
    }
    public void setIntent(DisconnectIntent intent) {
        this.intent = intent;
    }
    public void setWillReconnect(Boolean willReconnect) {
        this.willReconnect = willReconnect;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof DisconnectInfo) {
            DisconnectInfo t = (DisconnectInfo) other;
            return (
              Objects.equals(code, t.code) && 
              
              Objects.equals(reason, t.reason) && 
              
              Objects.equals(intent, t.intent) && 
              
              Objects.equals(willReconnect, t.willReconnect)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(code, reason, intent, willReconnect);
    }
}


