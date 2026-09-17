package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;

import java.util.Collections;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;
import java.util.logging.Level;
import java.util.logging.Logger;

/**
 * Idiomatic Java wrapper for WebSocket client with dual streaming patterns.
 *
 * <p>Supports two modes of operation:
 * <ul>
 *   <li><b>Callback mode:</b> Provide a WebSocketListener for push-based events</li>
 *   <li><b>Pull mode:</b> Use poll()/take() to consume messages from a BlockingQueue</li>
 * </ul>
 *
 * <h3>Example - Callback Mode:</h3>
 * <pre>{@code
 * WebSocketListener listener = new WebSocketListener() {
 *     public void onConnected() {
 *         System.out.println("Connected!");
 *     }
 *     public void onAuthenticated(String dataJson) {
 *         System.out.println("Authenticated");
 *     }
 *     public void onUnauthenticated(String dataJson) {
 *         System.err.println("Rejected: " + dataJson);
 *     }
 *     public void onDisconnected(Boolean willReconnect) {
 *         System.out.println("Disconnected (will reconnect: " + willReconnect + ")");
 *     }
 *     public void onMessage(StreamMessage message) {
 *         System.out.println("Event: " + message.event());
 *     }
 *     public void onError(ErrorInfo error) {
 *         System.err.println("Error: " + error.message());
 *     }
 * };
 *
 * try (FugleWebSocketClient client = FugleWebSocketClient.builder()
 *         .apiKey("YOUR_API_KEY")
 *         .stock()
 *         .listener(listener)
 *         .build()) {
 *
 *     client.connect().get();
 *     client.subscribe("trades", "2330").get();
 *     Thread.sleep(10000);
 * }
 * }</pre>
 *
 * <h3>Example - Pull Mode:</h3>
 * <pre>{@code
 * try (FugleWebSocketClient client = FugleWebSocketClient.builder()
 *         .apiKey("YOUR_API_KEY")
 *         .stock()
 *         .queueCapacity(1000)
 *         .build()) {
 *
 *     client.connect().get();
 *     client.subscribe("trades", "2330").get();
 *
 *     while (true) {
 *         StreamMessage msg = client.poll(1, TimeUnit.SECONDS);
 *         if (msg != null) {
 *             System.out.println("Event: " + msg.event());
 *         }
 *
 *         // Check for async errors
 *         if (client.hasErrors()) {
 *             String error = client.pollError();
 *             System.err.println("Async error: " + error);
 *         }
 *     }
 * }
 * }</pre>
 */
public class FugleWebSocketClient implements AutoCloseable {

    private final WebSocketClient webSocketClient;
    private final BlockingQueue<StreamMessage> messageQueue;
    private final BlockingQueue<String> errorQueue;
    /** The pull-mode listener, or null in callback mode. */
    private final InternalListener pullListener;

    private FugleWebSocketClient(WebSocketClient webSocketClient,
                                  BlockingQueue<StreamMessage> messageQueue,
                                  BlockingQueue<String> errorQueue,
                                  InternalListener pullListener) {
        this.webSocketClient = webSocketClient;
        this.messageQueue = messageQueue;
        this.errorQueue = errorQueue;
        this.pullListener = pullListener;
    }

    /**
     * Connect to the WebSocket server.
     *
     * @return CompletableFuture that completes when connected
     * @throws ApiException if connection fails
     * @throws AuthException if authentication fails
     */
    public CompletableFuture<Void> connect() {
        if (pullListener != null) {
            pullListener.resume();
        }
        return webSocketClient.connect()
                .exceptionally(e -> { throw FugleException.unwrap(e); });
    }

    /**
     * Disconnect from the WebSocket server.
     *
     * @return CompletableFuture that completes when disconnected
     */
    public CompletableFuture<Void> disconnect() {
        if (pullListener != null) {
            pullListener.stop();
        }
        return webSocketClient.disconnect();
    }

    /**
     * Check if currently connected.
     */
    public boolean isConnected() {
        try {
            return webSocketClient.isConnected();
        } catch (Exception e) {
            throw FugleException.unwrap(e);
        }
    }

    /**
     * Check if the client has been shut down.
     */
    public boolean isClosed() {
        try {
            return webSocketClient.isClosed();
        } catch (Exception e) {
            throw FugleException.unwrap(e);
        }
    }

    /**
     * Messages dropped this connection because the message queue held
     * {@code messageBuffer} unread messages ({@link MessageOverflow#DROP_NEWEST}).
     *
     * <p>Counted from the start of the current connection (every {@link #connect()}
     * or reconnect restarts it); after {@link #disconnect()} it still reads the
     * last connection's count. 0 before the first {@code connect()}.
     */
    public long messagesDroppedTotal() {
        try {
            return webSocketClient.messagesDroppedTotal();
        } catch (Exception e) {
            throw FugleException.unwrap(e);
        }
    }

    /**
     * Send a ping message to the server.
     *
     * @param state Optional state string echoed back in the pong response (nullable)
     * @return CompletableFuture that completes when the ping is sent
     */
    public CompletableFuture<Void> ping(String state) {
        return webSocketClient.ping(state)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
    }

    /**
     * Query the server for current subscriptions.
     * The response arrives via the message callback or pull queue.
     *
     * @return CompletableFuture that completes when the query is sent
     */
    public CompletableFuture<Void> querySubscriptions() {
        return webSocketClient.querySubscriptions()
                .exceptionally(e -> { throw FugleException.unwrap(e); });
    }

    /**
     * Subscribe to a channel for a symbol.
     *
     * @param channel Channel name (e.g., "trades", "candles", "books")
     * @param symbol Symbol to subscribe (e.g., "2330")
     * @return CompletableFuture that completes when subscribed
     * @throws ApiException if subscription fails
     */
    public CompletableFuture<Void> subscribe(String channel, String symbol) {
        return webSocketClient.subscribe(channel, symbol)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
    }

    /**
     * Unsubscribe from a channel for a symbol.
     *
     * @param channel Channel name
     * @param symbol Symbol to unsubscribe
     * @return CompletableFuture that completes when unsubscribed
     */
    public CompletableFuture<Void> unsubscribe(String channel, String symbol) {
        return webSocketClient.unsubscribe(channel, symbol)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
    }

    /**
     * Poll for the next message (non-blocking).
     *
     * <p>Only available in pull mode (when listener was not provided).
     *
     * @return Next message, or null if queue is empty
     */
    public StreamMessage poll() {
        if (messageQueue == null) {
            throw new IllegalStateException("poll() only available in pull mode (no listener provided)");
        }
        return messageQueue.poll();
    }

    /**
     * Poll for the next message with timeout.
     *
     * <p>Only available in pull mode (when listener was not provided).
     *
     * @param timeout Maximum time to wait
     * @param unit Time unit for timeout
     * @return Next message, or null if timeout expires
     * @throws InterruptedException if interrupted while waiting
     */
    public StreamMessage poll(long timeout, TimeUnit unit) throws InterruptedException {
        if (messageQueue == null) {
            throw new IllegalStateException("poll() only available in pull mode (no listener provided)");
        }
        return messageQueue.poll(timeout, unit);
    }

    /**
     * Take the next message (blocking).
     *
     * <p>Only available in pull mode (when listener was not provided).
     *
     * @return Next message (blocks until available)
     * @throws InterruptedException if interrupted while waiting
     */
    public StreamMessage take() throws InterruptedException {
        if (messageQueue == null) {
            throw new IllegalStateException("take() only available in pull mode (no listener provided)");
        }
        return messageQueue.take();
    }

    /**
     * Get the current message queue size.
     *
     * <p>Only available in pull mode (when listener was not provided).
     *
     * @return Number of messages in queue
     */
    public int queueSize() {
        if (messageQueue == null) {
            throw new IllegalStateException("queueSize() only available in pull mode (no listener provided)");
        }
        return messageQueue.size();
    }

    /**
     * Check if any async errors have occurred.
     *
     * <p>Only available in pull mode (when listener was not provided).
     *
     * @return true if errors are in the error queue
     */
    public boolean hasErrors() {
        if (errorQueue == null) {
            throw new IllegalStateException("hasErrors() only available in pull mode (no listener provided)");
        }
        return !errorQueue.isEmpty();
    }

    /**
     * Poll for the next error (non-blocking).
     *
     * <p>Only available in pull mode (when listener was not provided).
     *
     * @return Next error message, or null if no errors
     */
    public String pollError() {
        if (errorQueue == null) {
            throw new IllegalStateException("pollError() only available in pull mode (no listener provided)");
        }
        return errorQueue.poll();
    }

    @Override
    public void close() {
        if (pullListener != null) {
            pullListener.stop();
        }
        webSocketClient.close();
    }

    /**
     * Create a new builder.
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for FugleWebSocketClient.
     */
    public static class Builder {
        private String apiKey;
        private String bearerToken;
        private String sdkToken;
        private String baseUrl;
        private WebSocketEndpoint endpoint = WebSocketEndpoint.STOCK;
        private WebSocketListener listener;
        private int queueCapacity = 10000;
        private ReconnectOptions reconnectOptions;
        private HealthCheckOptions healthCheckOptions;
        private MessageOverflow messageOverflow;
        private Integer messageBuffer;

        private Builder() {}

        /**
         * Set API key for authentication.
         */
        public Builder apiKey(String apiKey) {
            this.apiKey = apiKey;
            return this;
        }

        /**
         * Set bearer token for OAuth authentication.
         */
        public Builder bearerToken(String bearerToken) {
            this.bearerToken = bearerToken;
            return this;
        }

        /**
         * Set SDK token for legacy authentication.
         */
        public Builder sdkToken(String sdkToken) {
            this.sdkToken = sdkToken;
            return this;
        }

        /**
         * Set custom base URL for WebSocket endpoint.
         *
         * @param baseUrl Custom base URL (e.g., "wss://custom.ws.fugle.tw")
         * @return This builder for chaining
         */
        public Builder baseUrl(String baseUrl) {
            this.baseUrl = baseUrl;
            return this;
        }

        /**
         * Use stock market data endpoint (default).
         */
        public Builder stock() {
            this.endpoint = WebSocketEndpoint.STOCK;
            return this;
        }

        /**
         * Use futures and options market data endpoint.
         */
        public Builder futopt() {
            this.endpoint = WebSocketEndpoint.FUT_OPT;
            return this;
        }

        /**
         * Use futures and options market data endpoint (alias).
         */
        public Builder futOpt() {
            return futopt();
        }

        /**
         * Provide a listener for callback mode (push-based events).
         *
         * <p>If a listener is provided, poll()/take() methods will throw IllegalStateException.
         *
         * <p>The listener is wrapped in {@link SafeListener} (#83): an
         * exception thrown by any of its methods (other than {@code onError})
         * is caught, reported to its own {@code onError} as a code 3004
         * {@code CALLBACK_FAILED} error, and the stream keeps running.
         */
        public Builder listener(WebSocketListener listener) {
            this.listener = listener == null ? null : new SafeListener(listener);
            return this;
        }

        /**
         * Set message queue capacity for pull mode.
         *
         * <p>Default: 10000
         * <p>Only used when no listener is provided.
         */
        public Builder queueCapacity(int capacity) {
            this.queueCapacity = capacity;
            return this;
        }

        /**
         * Set reconnection options for WebSocket client.
         *
         * @param reconnectOptions Reconnection configuration
         * @return This builder for chaining
         */
        public Builder reconnect(ReconnectOptions reconnectOptions) {
            this.reconnectOptions = reconnectOptions;
            return this;
        }

        /**
         * Set health check options for WebSocket client.
         *
         * @param healthCheckOptions Health check configuration
         * @return This builder for chaining
         */
        public Builder healthCheck(HealthCheckOptions healthCheckOptions) {
            this.healthCheckOptions = healthCheckOptions;
            return this;
        }

        /**
         * Set what happens to an inbound message while the message queue
         * already holds {@code messageBuffer} unread messages.
         *
         * <p>Default: {@link MessageOverflow#DROP_NEWEST}.
         *
         * @param messageOverflow Overflow behavior
         * @return This builder for chaining
         */
        public Builder messageOverflow(MessageOverflow messageOverflow) {
            this.messageOverflow = messageOverflow;
            return this;
        }

        /**
         * Set how many unread messages the message queue holds before
         * {@code messageOverflow} applies.
         *
         * <p>Default: 4096.
         *
         * @param messageBuffer Must be greater than 0
         * @return This builder for chaining
         * @throws IllegalArgumentException if messageBuffer is not greater than 0
         */
        public Builder messageBuffer(int messageBuffer) {
            if (messageBuffer <= 0) {
                throw new IllegalArgumentException("messageBuffer must be > 0");
            }
            this.messageBuffer = messageBuffer;
            return this;
        }

        /**
         * Build the FugleWebSocketClient.
         *
         * @throws FugleException with code 1004 if not exactly one non-empty
         *     authentication method is provided (empty or whitespace-only values
         *     count as not provided)
         */
        public FugleWebSocketClient build() {
            WebSocketListener effectiveListener;
            BlockingQueue<StreamMessage> messageQueue;
            BlockingQueue<String> errorQueue;
            InternalListener pullListener;

            if (listener != null) {
                // Callback mode: use provided listener directly
                effectiveListener = listener;
                messageQueue = null;
                errorQueue = null;
                pullListener = null;
            } else {
                // Pull mode: create internal listener with BlockingQueue
                messageQueue = new LinkedBlockingQueue<>(queueCapacity);
                errorQueue = new LinkedBlockingQueue<>();

                pullListener = new InternalListener(messageQueue, errorQueue);
                effectiveListener = pullListener;
            }

            // Convert config options to UniFFI record types
            ReconnectConfigRecord reconnectRecord = null;
            if (reconnectOptions != null) {
                reconnectRecord = new ReconnectConfigRecord(
                    reconnectOptions.getMaxAttempts() != null ? reconnectOptions.getMaxAttempts() : 0,
                    reconnectOptions.getInitialDelayMs() != null ? reconnectOptions.getInitialDelayMs() : 0L,
                    reconnectOptions.getMaxDelayMs() != null ? reconnectOptions.getMaxDelayMs() : 0L
                );
            }

            HealthCheckConfigRecord healthCheckRecord = null;
            if (healthCheckOptions != null) {
                // Unset values map to the core defaults: enabled, and a
                // heartbeat timeout of 0 meaning "use 35000 ms".
                healthCheckRecord = new HealthCheckConfigRecord(
                    healthCheckOptions.getEnabled() != null ? healthCheckOptions.getEnabled() : true,
                    healthCheckOptions.getHeartbeatTimeoutMs() != null ? healthCheckOptions.getHeartbeatTimeoutMs() : 0L
                );
            }

            // Unset overflow/buffer both mean "use the core defaults"
            // (DropNewest, 4096), so leave the whole record null then.
            MessageQueueConfigRecord messageQueueRecord = null;
            if (messageOverflow != null || messageBuffer != null) {
                MessageOverflowRecord overflowRecord = messageOverflow != null
                    ? MessageOverflowRecord.valueOf(messageOverflow.name())
                    : MessageOverflowRecord.DROP_NEWEST;
                messageQueueRecord = new MessageQueueConfigRecord(
                    overflowRecord,
                    messageBuffer != null ? messageBuffer : 0
                );
            }

            // Core requires exactly one non-blank credential (ConfigError,
            // code 1004) and sends it in the auth frame as apikey, token or
            // sdkToken to match its kind.
            WebSocketClient client;
            try {
                client = WebSocketClient.newWithCredentials(
                    new CredentialsRecord(apiKey, bearerToken, sdkToken), effectiveListener, endpoint, baseUrl,
                    reconnectRecord, healthCheckRecord, null, null, messageQueueRecord
                );
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }

            return new FugleWebSocketClient(client, messageQueue, errorQueue, pullListener);
        }
    }

    /**
     * Wraps a user-provided {@link WebSocketListener} (callback mode only)
     * so an exception it throws cannot cross the FFI boundary (#83): it is
     * caught, reported to the listener's own {@code onError} as a code 3004
     * {@code CALLBACK_FAILED} error (throttled — see {@link ReportThrottle}),
     * and the stream keeps running. An exception from {@code onError}
     * itself — whether reporting a callback failure or a normal SDK error —
     * is neither re-thrown nor re-reported; it is logged at {@code WARNING}.
     *
     * <p>Pull mode's {@link InternalListener} is not wrapped: it is internal
     * to this class and does not call into user code.
     */
    static final class SafeListener implements WebSocketListener {
        /** Numeric code for a listener callback throwing (mirrors the Rust core's error_code table). */
        static final int CALLBACK_FAILED_CODE = 3004;

        private static final Logger LOGGER = Logger.getLogger(FugleWebSocketClient.class.getName());

        private final WebSocketListener delegate;
        private final ReportThrottle throttle;

        SafeListener(WebSocketListener delegate) {
            this(delegate, new ReportThrottle());
        }

        /** Test-only constructor to inject the throttle's clock. */
        SafeListener(WebSocketListener delegate, ReportThrottle throttle) {
            this.delegate = delegate;
            this.throttle = throttle;
        }

        @Override
        public void onConnected() {
            invoke("onConnected", delegate::onConnected);
        }

        @Override
        public void onAuthenticated(String dataJson) {
            invoke("onAuthenticated", () -> delegate.onAuthenticated(dataJson));
        }

        @Override
        public void onUnauthenticated(String dataJson) {
            invoke("onUnauthenticated", () -> delegate.onUnauthenticated(dataJson));
        }

        @Override
        public void onDisconnected(Boolean willReconnect) {
            invoke("onDisconnected", () -> delegate.onDisconnected(willReconnect));
        }

        @Override
        public void onMessage(StreamMessage message) {
            invoke("onMessage", () -> delegate.onMessage(message));
        }

        @Override
        public void onError(ErrorInfo error) {
            invokeOnError(() -> delegate.onError(error));
        }

        @Override
        public void onReconnecting(Integer attempt) {
            invoke("onReconnecting", () -> delegate.onReconnecting(attempt));
        }

        @Override
        public void onReconnectFailed(Integer attempts) {
            invoke("onReconnectFailed", () -> delegate.onReconnectFailed(attempts));
        }

        @Override
        public void onMessagesDropped(Long count) {
            invoke("onMessagesDropped", () -> delegate.onMessagesDropped(count));
        }

        /** Run a listener method; a thrown exception is reported via onError instead of crossing the FFI boundary. */
        private void invoke(String methodName, Runnable call) {
            try {
                call.run();
            } catch (Exception ex) {
                reportCallbackFailure(methodName, ex);
            }
        }

        /** Run onError itself; a thrown exception is logged, never re-thrown or re-reported. */
        private void invokeOnError(Runnable call) {
            try {
                call.run();
            } catch (Exception ex) {
                LOGGER.log(Level.WARNING, "WebSocketListener.onError threw", ex);
            }
        }

        private void reportCallbackFailure(String methodName, Exception ex) {
            Long count = throttle.record();
            if (count == null) {
                return;
            }

            String message = "Listener " + methodName + " threw " + ex.getClass().getName()
                    + ": " + ex.getMessage() + " (" + count + " in the last 1s)";
            ErrorInfo error = new ErrorInfo(
                    CALLBACK_FAILED_CODE,
                    ErrorSourceKind.CLIENT,
                    message,
                    null,
                    null,
                    null,
                    Collections.emptyMap()
            );

            invokeOnError(() -> delegate.onError(error));
        }
    }

    /**
     * Internal listener for pull mode that forwards events to BlockingQueues.
     *
     * <p>A full message queue holds the SDK's delivery until {@code poll()}
     * makes room, so messages you do not keep up with are dropped (per
     * {@code messageOverflow}), counted in {@code messagesDroppedTotal()} and
     * reported on the error queue by the SDK itself, never silently here.
     * {@code disconnect()} and {@code close()} end the wait.
     */
    static class InternalListener implements WebSocketListener {
        /** How often a wait for queue room re-checks for disconnect/close. */
        private static final long WAIT_SLICE_MS = 100;

        private final BlockingQueue<StreamMessage> messageQueue;
        private final BlockingQueue<String> errorQueue;
        /**
         * The current connection's stop flag. A new one per connection, so a
         * wait from before {@code disconnect()} still ends when
         * {@code connect()} is called right after it.
         */
        private final AtomicReference<AtomicBoolean> stopped =
                new AtomicReference<>(new AtomicBoolean(false));

        InternalListener(BlockingQueue<StreamMessage> messageQueue,
                        BlockingQueue<String> errorQueue) {
            this.messageQueue = messageQueue;
            this.errorQueue = errorQueue;
        }

        /** Stop waiting for queue room: the connection is being closed. */
        void stop() {
            stopped.get().set(true);
        }

        /** A new connection: wait for queue room again. */
        void resume() {
            stopped.set(new AtomicBoolean(false));
        }

        @Override
        public void onConnected() {
            // No action needed in pull mode
        }

        @Override
        public void onAuthenticated(String dataJson) {
            // No action needed in pull mode
        }

        @Override
        public void onUnauthenticated(String dataJson) {
            // Credential rejection surfaces on the error queue, as it did
            // when it was reported through onError.
            errorQueue.offer("Unauthenticated: " + (dataJson == null ? "" : dataJson));
        }

        @Override
        public void onDisconnected(Boolean willReconnect) {
            // No action needed in pull mode
        }

        @Override
        public void onMessage(StreamMessage message) {
            // Wait for room: holding the SDK here leaves the backlog to its
            // own queue, which drops and reports per messageOverflow.
            AtomicBoolean connectionStopped = stopped.get();
            try {
                while (!connectionStopped.get()) {
                    if (messageQueue.offer(message, WAIT_SLICE_MS, TimeUnit.MILLISECONDS)) {
                        return;
                    }
                }
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
        }

        @Override
        public void onError(ErrorInfo error) {
            // Offer the human-readable message to the error queue, same as
            // before this callback carried a structured ErrorInfo. Callers
            // that need `code` / `sourceKind` / HTTP details should switch
            // to callback mode and read them off the ErrorInfo directly.
            errorQueue.offer(error.message());
        }

        @Override
        public void onReconnecting(Integer attempt) {
            // No action needed in pull mode
        }

        @Override
        public void onReconnectFailed(Integer attempts) {
            errorQueue.offer("All " + attempts + " reconnection attempts exhausted");
        }

        @Override
        public void onMessagesDropped(Long count) {
            // Reported through the error queue, like onUnauthenticated and
            // onReconnectFailed; there is no dedicated queue for this event.
            errorQueue.offer("Dropped " + count + " message(s): listener fell behind");
        }
    }
}
