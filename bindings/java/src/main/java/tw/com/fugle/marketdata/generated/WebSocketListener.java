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
 * public void OnDisconnected() {
 * Console.WriteLine("Disconnected");
 * }
 * public void OnMessage(StreamMessage message) {
 * Console.WriteLine($"Got {message.Event} for {message.Symbol}");
 * }
 * public void OnError(string errorMessage) {
 * Console.WriteLine($"Error: {errorMessage}");
 * }
 * }
 * ```
 */
public interface WebSocketListener {
    
    /**
     * Called when WebSocket connection is established
     */
    public void onConnected();
    
    /**
     * Called when WebSocket connection is closed
     */
    public void onDisconnected();
    
    /**
     * Called when a message is received
     */
    public void onMessage(StreamMessage message);
    
    /**
     * Called when an error occurs
     */
    public void onError(String errorMessage);
    
    /**
     * Called when a reconnection attempt starts
     */
    public void onReconnecting(Integer attempt);
    
    /**
     * Called when all reconnection attempts are exhausted
     */
    public void onReconnectFailed(Integer attempts);
    
}

