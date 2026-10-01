package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.CredentialsRecord;

/**
 * One credential to authenticate with: an API key, a bearer token or an SDK
 * token, for {@link FugleWebSocketClient#setCredentials(Credentials)} and
 * {@link FugleRestClient#setCredentials(Credentials)} (#322).
 *
 * <p>A blank value is refused when it is used, with {@link FugleException}
 * code 1004, as at construction. {@link #toString()} does not print the
 * secret.
 *
 * <pre>{@code
 * ws.setCredentials(Credentials.sdkToken(newToken));
 * }</pre>
 */
public final class Credentials {

    private final String apiKey;
    private final String bearerToken;
    private final String sdkToken;

    private Credentials(String apiKey, String bearerToken, String sdkToken) {
        this.apiKey = apiKey;
        this.bearerToken = bearerToken;
        this.sdkToken = sdkToken;
    }

    /** A Fugle API key, sent as {@code apikey}. */
    public static Credentials apiKey(String apiKey) {
        return new Credentials(apiKey, null, null);
    }

    /** An OAuth bearer token, sent as {@code token}. */
    public static Credentials bearerToken(String bearerToken) {
        return new Credentials(null, bearerToken, null);
    }

    /** A Fugle SDK token, sent as {@code sdkToken}. */
    public static Credentials sdkToken(String sdkToken) {
        return new Credentials(null, null, sdkToken);
    }

    /** The record the native client takes. */
    CredentialsRecord toRecord() {
        return new CredentialsRecord(apiKey, bearerToken, sdkToken);
    }

    @Override
    public String toString() {
        String kind = apiKey != null ? "apiKey" : bearerToken != null ? "bearerToken" : "sdkToken";
        return "Credentials." + kind + "(***)";
    }
}
