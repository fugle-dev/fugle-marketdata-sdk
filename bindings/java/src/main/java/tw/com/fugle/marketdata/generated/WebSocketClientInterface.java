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
    
    /**
     * Disconnect, returning once the listener has handled the connection's
     * remaining events, `on_disconnected` included.
     *
     * There is no timeout on that wait: a listener method that blocks keeps
     * `disconnect()` waiting for as long as it does.
     *
     * Called from a listener method, it returns without that wait: those
     * events are delivered on the thread running the method, after it
     * returns.
     */
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
    
    /**
     * Subscribe to a channel for a symbol.
     *
     * After-hours (盤後) is FutOpt only: on the Stock endpoint, any value
     * other than null is 1005 `INVALID_PARAMETER`.
     */
    public CompletableFuture<Void> subscribe(String channel, String symbol, Boolean afterHours) ;
    
    /**
     * Unsubscribe from a channel for a symbol.
     *
     * Pass the same after-hours value as the `subscribe` call: an after-hours
     * subscription is a separate subscription from the regular one.
     */
    public CompletableFuture<Void> unsubscribe(String channel, String symbol, Boolean afterHours) ;
    
    /**
     * Unsubscribe by the ids the server issued in its `subscribed` messages.
     *
     * Removes the subscriptions those ids name, so a reconnect does not
     * restore them. An empty list is 1005 `INVALID_PARAMETER`.
     */
    public CompletableFuture<Void> unsubscribeIds(List<String> ids) ;
    
}

