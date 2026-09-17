package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Message queue configuration record for FFI
 *
 * `buffer` is 0 for the default (4096).
 */
public class MessageQueueConfigRecord {
    /**
     * What happens to new messages while `buffer` are unread
     */
    private MessageOverflowRecord overflow;
    /**
     * Unread messages held (default 4096; 0 means default)
     */
    private Integer buffer;

    public MessageQueueConfigRecord(
        MessageOverflowRecord overflow, 
        Integer buffer
    ) {
        
        this.overflow = overflow;
        
        this.buffer = buffer;
    }
    
    public MessageOverflowRecord overflow() {
        return this.overflow;
    }
    
    public Integer buffer() {
        return this.buffer;
    }
    public void setOverflow(MessageOverflowRecord overflow) {
        this.overflow = overflow;
    }
    public void setBuffer(Integer buffer) {
        this.buffer = buffer;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof MessageQueueConfigRecord) {
            MessageQueueConfigRecord t = (MessageQueueConfigRecord) other;
            return (
              Objects.equals(overflow, t.overflow) && 
              
              Objects.equals(buffer, t.buffer)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(overflow, buffer);
    }
}


