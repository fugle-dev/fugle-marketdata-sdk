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
     * Measure the round trip to the server: send a ping, wait for its pong,
     * and return the time between the two in milliseconds.
     *
     * Unlike `ping()` (fire and forget, pong delivered to `on_message`),
     * this waits for the answer, and its pong is not delivered. Works
     * whether or not `probe_enabled` is set, and sends nothing in the
     * background. `timeout_ms` defaults to 5000 when `None`.
     *
     * Errors: `ClientClosed` (2010) when not connected, `ConnectionError`
     * (2001) when the connection closes before the pong, `TimeoutError`
     * (3001) when no pong arrives within `timeout_ms`, and
     * `InvalidParameter` (1005) for a `timeout_ms` of 0.
     */
    public CompletableFuture<Double> measureLatency(Long timeoutMs) ;
    
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
     * Subscribe to a channel for one or more symbols.
     *
     * One symbol is sent as `symbol`, several as `symbols` in one frame;
     * each symbol is its own subscription afterwards. An empty list is
     * 1005 `INVALID_PARAMETER`.
     *
     * `opts` selects the session: `intraday_odd_lot` is Stock only and
     * `after_hours` is FutOpt only; setting either on the other endpoint,
     * to any value, is 1005 `INVALID_PARAMETER`.
     */
    public CompletableFuture<Void> subscribe(String channel, List<String> symbols, SubscribeOptions opts) ;
    
    /**
     * Unsubscribe from a channel for one or more symbols.
     *
     * Pass the same options as the `subscribe` call: an odd-lot or
     * after-hours subscription is a separate subscription from the regular
     * one.
     */
    public CompletableFuture<Void> unsubscribe(String channel, List<String> symbols, SubscribeOptions opts) ;
    
    /**
     * Unsubscribe by the ids the server issued in its `subscribed` messages.
     *
     * Removes the subscriptions those ids name, so a reconnect does not
     * restore them. An empty list is 1005 `INVALID_PARAMETER`.
     */
    public CompletableFuture<Void> unsubscribeIds(List<String> ids) ;
    
}

