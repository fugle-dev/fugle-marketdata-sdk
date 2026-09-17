package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * The credentials a WebSocket client authenticates with.
 *
 * Exactly one must be non-empty; an empty or whitespace-only value counts
 * as not provided.
 *
 * Its fields are secrets: do not log this record. `Debug` here redacts
 * them, but the generated types may not — a C# record's `ToString()` and
 * Go's `fmt` `%v` print every field.
 */
public class CredentialsRecord {
    /**
     * Fugle API key, sent as `apikey`
     */
    private String apiKey;
    /**
     * OAuth bearer token, sent as `token`
     */
    private String bearerToken;
    /**
     * Fugle SDK token, sent as `sdkToken`
     */
    private String sdkToken;

    public CredentialsRecord(
        String apiKey, 
        String bearerToken, 
        String sdkToken
    ) {
        
        this.apiKey = apiKey;
        
        this.bearerToken = bearerToken;
        
        this.sdkToken = sdkToken;
    }
    
    public String apiKey() {
        return this.apiKey;
    }
    
    public String bearerToken() {
        return this.bearerToken;
    }
    
    public String sdkToken() {
        return this.sdkToken;
    }
    public void setApiKey(String apiKey) {
        this.apiKey = apiKey;
    }
    public void setBearerToken(String bearerToken) {
        this.bearerToken = bearerToken;
    }
    public void setSdkToken(String sdkToken) {
        this.sdkToken = sdkToken;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof CredentialsRecord) {
            CredentialsRecord t = (CredentialsRecord) other;
            return (
              Objects.equals(apiKey, t.apiKey) && 
              
              Objects.equals(bearerToken, t.bearerToken) && 
              
              Objects.equals(sdkToken, t.sdkToken)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(apiKey, bearerToken, sdkToken);
    }
}


