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
     * Check if the client has been shut down
     */
    public Boolean isClosed();
    
    /**
     * Check if the client is currently connected
     */
    public Boolean isConnected();
    
    public CompletableFuture<Void> ping(String state) ;
    
    public CompletableFuture<Void> querySubscriptions() ;
    
    public CompletableFuture<Void> subscribe(String channel, String symbol) ;
    
    public CompletableFuture<Void> unsubscribe(String channel, String symbol) ;
    
}

