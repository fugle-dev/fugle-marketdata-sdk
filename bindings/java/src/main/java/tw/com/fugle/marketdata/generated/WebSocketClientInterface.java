package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * WebSocket client for real-time market data streaming
 *
 * Wraps the core WebSocketClient and forwards messages to the provided
 * WebSocketListener implementation via a background task.
 */
public interface WebSocketClientInterface {
    
    public CompletableFuture<Void> connect() ;
    
    public CompletableFuture<Void> disconnect();
    
    /**
     * Check if the connection has ended
     *
     * Reads core's connection state: true after `disconnect()`, and after
     * the server closes the connection when no reconnect follows (disabled
     * or attempts exhausted). False while reconnecting and before the first
     * `connect()`.
     */
    public Boolean isClosed();
    
    /**
     * Check if the client is currently connected
     *
     * Reads core's connection state, so it is false while reconnecting and
     * right after the connection drops, without waiting for the event thread.
     */
    public Boolean isConnected();
    
    /**
     * Messages dropped because they arrived while the message queue held
     * `buffer` unread messages (`MessageOverflowRecord::DropNewest`).
     *
     * Counted from the start of the current connection (every `connect()` or
     * reconnect restarts it); after `disconnect()` it still reads the last
     * connection's count. 0 before the first `connect()`.
     */
    public Long messagesDroppedTotal();
    
    public CompletableFuture<Void> ping(String state) ;
    
    public CompletableFuture<Void> querySubscriptions() ;
    
    public CompletableFuture<Void> subscribe(String channel, String symbol) ;
    
    public CompletableFuture<Void> unsubscribe(String channel, String symbol) ;
    
}

