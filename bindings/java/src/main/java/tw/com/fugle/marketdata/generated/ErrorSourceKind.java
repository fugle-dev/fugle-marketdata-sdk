package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;

/**
 * Coarse-grained classification of the source of a [`MarketDataError`].
 *
 * Mirrors `marketdata_core::ErrorKind`. That core enum is `#[non_exhaustive]`
 * so a future variant this crate doesn't know about yet maps to `Client`
 * (see the `From` impl below) rather than failing to compile.
 */

public enum ErrorSourceKind {
    /**
     * Transport-level transient failure: connection reset, timeout,
     * heartbeat gap, server outage (5xx). Generally safe to retry with
     * backoff.
     */
  NETWORK,
    /**
     * Protocol-level violation or unclassified WebSocket failure. Indicates
     * an SDK / version mismatch or a server-side bug; retry is unlikely to
     * help.
     */
  PROTOCOL,
    /**
     * Authentication / authorization failure: bad credentials, 401/403,
     * expired token, TLS cert failure. Human intervention required.
     */
  AUTH,
    /**
     * Server is rejecting requests because the caller is exceeding its
     * rate budget (HTTP 429).
     */
  RATE_LIMIT,
    /**
     * Caller-side problem: invalid input, configuration error, client
     * already closed, serialization failure, non-auth/non-throttle 4xx.
     */
  CLIENT;
}


