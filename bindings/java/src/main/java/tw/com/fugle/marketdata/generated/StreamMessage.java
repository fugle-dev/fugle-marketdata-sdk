package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Streaming message (simplified for FFI)
 */
public class StreamMessage {
    private String event;
    private String channel;
    private String symbol;
    private String id;
    private String dataJson;
    private Integer errorCode;
    private String errorMessage;

    public StreamMessage(
        String event, 
        String channel, 
        String symbol, 
        String id, 
        String dataJson, 
        Integer errorCode, 
        String errorMessage
    ) {
        
        this.event = event;
        
        this.channel = channel;
        
        this.symbol = symbol;
        
        this.id = id;
        
        this.dataJson = dataJson;
        
        this.errorCode = errorCode;
        
        this.errorMessage = errorMessage;
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
        return Objects.hash(event, channel, symbol, id, dataJson, errorCode, errorMessage);
    }
}


