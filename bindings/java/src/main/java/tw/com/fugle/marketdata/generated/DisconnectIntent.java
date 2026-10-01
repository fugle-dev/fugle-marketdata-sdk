package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;

/**
 * Who closed the connection, in a [`DisconnectInfo`] (#293).
 */

public enum DisconnectIntent {
    /**
     * Your `disconnect()`.
     */
  CLIENT,
    /**
     * The server's Close frame, whatever its code.
     */
  SERVER,
    /**
     * Transport error, EOF without a Close frame, or heartbeat timeout.
     */
  NETWORK;
}


