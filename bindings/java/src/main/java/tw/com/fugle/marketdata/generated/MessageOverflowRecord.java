package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;

/**
 * What the client does with an inbound message while its queue already
 * holds `buffer` unread messages.
 */

public enum MessageOverflowRecord {
    /**
     * Drop new messages and report them through `on_messages_dropped`.
     */
  DROP_NEWEST,
    /**
     * Never drop: the queue grows while `on_message` lags.
     */
  UNBOUNDED;
}


