package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * An inbound streaming frame.
 *
 * `raw` is the frame exactly as the server sent it — decode that when you
 * want the payload. The other fields are the routing subset this SDK parses
 * out so callbacks can dispatch without decoding the whole frame first; they
 * are a convenience, not the source of truth.
 */
public class StreamMessage {
    /**
     * The frame verbatim, as received on the wire.
     */
    private String raw;
    /**
     * Event type: "data", "subscribed", "error", "authenticated", "pong".
     */
    private String event;
    /**
     * Channel name, for data events.
     */
    private String channel;
    /**
     * Symbol, for data events.
     */
    private String symbol;
    /**
     * Subscription id, for subscribed events.
     */
    private String id;
    /**
     * The `data` member of the frame, still encoded as JSON.
     */
    private String dataJson;
    /**
     * Server error code, for error events: `1000` credentials rejected,
     * `1001` subscription limit exceeded, `1002` command before
     * authentication, `1003` request validation failed, `1004` no auth
     * request within 60 s, `1011` auth service unavailable. Absent when the
     * server sent an error frame without a code.
     */
    private Integer errorCode;
    /**
     * Error message, for error events: the frame's `data.message`, or its
     * top-level `message` when the server sent the code-less shape.
     */
    private String errorMessage;

    public StreamMessage(
        String raw, 
        String event, 
        String channel, 
        String symbol, 
        String id, 
        String dataJson, 
        Integer errorCode, 
        String errorMessage
    ) {
        
        this.raw = raw;
        
        this.event = event;
        
        this.channel = channel;
        
        this.symbol = symbol;
        
        this.id = id;
        
        this.dataJson = dataJson;
        
        this.errorCode = errorCode;
        
        this.errorMessage = errorMessage;
    }
    
    public String raw() {
        return this.raw;
    }
    
    public String event() {
        return this.event;
    }
    
    public String channel() {
        return this.channel;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String id() {
        return this.id;
    }
    
    public String dataJson() {
        return this.dataJson;
    }
    
    public Integer errorCode() {
        return this.errorCode;
    }
    
    public String errorMessage() {
        return this.errorMessage;
    }
    public void setRaw(String raw) {
        this.raw = raw;
    }
    public void setEvent(String event) {
        this.event = event;
    }
    public void setChannel(String channel) {
        this.channel = channel;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setId(String id) {
        this.id = id;
    }
    public void setDataJson(String dataJson) {
        this.dataJson = dataJson;
    }
    public void setErrorCode(Integer errorCode) {
        this.errorCode = errorCode;
    }
    public void setErrorMessage(String errorMessage) {
        this.errorMessage = errorMessage;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StreamMessage) {
            StreamMessage t = (StreamMessage) other;
            return (
              Objects.equals(raw, t.raw) && 
              
              Objects.equals(event, t.event) && 
              
              Objects.equals(channel, t.channel) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(id, t.id) && 
              
              Objects.equals(dataJson, t.dataJson) && 
              
              Objects.equals(errorCode, t.errorCode) && 
              
              Objects.equals(errorMessage, t.errorMessage)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(raw, event, channel, symbol, id, dataJson, errorCode, errorMessage);
    }
}


