package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * Callback interface for WebSocket events
 *
 * Foreign code (C#, Go) implements this trait to receive WebSocket events.
 * The implementation must be thread-safe (Send + Sync) as callbacks may be
 * invoked from background tokio tasks.
 *
 * # Example (C#)
 *
 * ```csharp
 * class MyListener : IWebSocketListener {
 * public void OnConnected() {
 * Console.WriteLine("Connected!");
 * }
 * public void OnAuthenticated(string? dataJson) {
 * Console.WriteLine("Authenticated");
 * }
 * public void OnUnauthenticated(string? dataJson) {
 * Console.WriteLine($"Rejected: {dataJson}");
 * }
 * public void OnDisconnected(bool willReconnect) {
 * Console.WriteLine($"Disconnected (will reconnect: {willReconnect})");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(ErrorInfo error) {
 * Console.WriteLine($"Error: {error.Message}");
 * }
 * }
 * ```
 */
public interface WebSocketListener {
    
    /**
     * Called when the transport is established, before the server has
     * answered the auth frame. Fires again on every successful reconnect.
     * Wait for `on_authenticated` before treating the connection as usable.
     */
    public void onConnected();
    
    /**
     * Called when the server accepts the credentials.
     *
     * `data_json` is the `data` member of the server's `authenticated`
     * frame, still encoded as JSON, or `None` when the frame has none.
     */
    public void onAuthenticated(String dataJson);
    
    /**
     * Called when the server rejects the credentials: it answered the auth
     * frame with an `error` of code 1000. On `connect()` the call also
     * fails with an auth error; no `on_error` is emitted for the rejection.
     * During an auto-reconnect, `on_reconnect_failed` follows at once: the
     * same credentials would be rejected again, so the client stops and
     * stays closed (#201). An auth-phase `error` with any other code (1011
     * auth service unavailable, 1004 no auth request received) is not a
     * rejection: it is reported to `on_error` (code 2001) and a reconnect
     * goes on.
     *
     * `data_json` is the `data` member of the server's rejection frame
     * (the server's message is under `message`), still encoded as JSON, or
     * `None` when the frame has none.
     */
    public void onUnauthenticated(String dataJson);
    
    /**
     * Called when the connection is closed, at most once per connection.
     *
     * `will_reconnect` is `true` when the client will try to reconnect
     * (`on_reconnecting` follows unless `disconnect()` is called first) and
     * `false` when this connection's lifecycle has ended.
     */
    public void onDisconnected(Boolean willReconnect);
    
    /**
     * Called when a message is received
     */
    public void onMessage(StreamMessage message);
    
    /**
     * Called when an error occurs
     *
     * Also carries one warning, code 3006 (`RECONNECT_CONFLICT`), at most
     * once per client: `disconnect()` closed a connection that automatic
     * reconnect had restored less than 30 seconds earlier, which is what
     * code that also reconnects on its own does (#226). The close goes
     * ahead; the message says how to resolve it.
     */
    public void onError(ErrorInfo error);
    
    /**
     * Called when a reconnection attempt starts
     */
    public void onReconnecting(Integer attempt);
    
    /**
     * Called when the reconnect gives up: all attempts are exhausted, or an
     * attempt's credentials were rejected (`on_unauthenticated` precedes
     * it, #201). Terminal: no further lifecycle callbacks follow for this
     * connection.
     */
    public void onReconnectFailed(Integer attempts);
    
    /**
     * Called when messages were dropped because `on_message` fell behind
     * while the client's message queue held `buffer` unread messages
     * (`MessageOverflowRecord::DropNewest`).
     *
     * `count` is the number dropped since the previous call. The first drop
     * on a connection is reported at once, later ones at most once per
     * second, and the rest before `on_disconnected`. The connection's total
     * is `WebSocketClient::messages_dropped_total()`.
     */
    public void onMessagesDropped(Long count);
    
}

