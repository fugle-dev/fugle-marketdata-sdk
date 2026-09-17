package tw.com.fugle.marketdata;

/**
 * What the client does with an inbound message while the message queue
 * already holds {@code messageBuffer} unread messages.
 *
 * <p>Used with {@link FugleWebSocketClient.Builder#messageOverflow} and
 * {@link FugleWebSocketClient.Builder#messageBuffer}.
 */
public enum MessageOverflow {
    /**
     * Drop new messages and report them through the listener's
     * {@code onMessagesDropped(count)} callback (default).
     */
    DROP_NEWEST,

    /**
     * Never drop: the queue grows while the listener falls behind.
     */
    UNBOUNDED;
}
