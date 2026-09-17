package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * The cross-language view of an error: the fields every binding exposes
 * under the same names. Mirrors `marketdata_core::ErrorInfo`.
 */
public class ErrorInfo {
    /**
     * Numeric code from `marketdata_core::error_code`, stable across
     * languages and releases.
     */
    private Integer code;
    /**
     * Category of the failure.
     */
    private ErrorSourceKind sourceKind;
    /**
     * Human-readable message.
     */
    private String message;
    /**
     * HTTP status, when the error came from an HTTP response (REST, or the
     * WebSocket upgrade).
     */
    private Short status;
    /**
     * Raw HTTP response body (REST only).
     */
    private String body;
    /**
     * Server-assigned request id (`x-request-id`), when present.
     */
    private String requestId;
    /**
     * HTTP response headers (REST only; empty otherwise).
     */
    private Map<String, String> headers;

    public ErrorInfo(
        Integer code, 
        ErrorSourceKind sourceKind, 
        String message, 
        Short status, 
        String body, 
        String requestId, 
        Map<String, String> headers
    ) {
        
        this.code = code;
        
        this.sourceKind = sourceKind;
        
        this.message = message;
        
        this.status = status;
        
        this.body = body;
        
        this.requestId = requestId;
        
        this.headers = headers;
    }
    
    public Integer code() {
        return this.code;
    }
    
    public ErrorSourceKind sourceKind() {
        return this.sourceKind;
    }
    
    public String message() {
        return this.message;
    }
    
    public Short status() {
        return this.status;
    }
    
    public String body() {
        return this.body;
    }
    
    public String requestId() {
        return this.requestId;
    }
    
    public Map<String, String> headers() {
        return this.headers;
    }
    public void setCode(Integer code) {
        this.code = code;
    }
    public void setSourceKind(ErrorSourceKind sourceKind) {
        this.sourceKind = sourceKind;
    }
    public void setMessage(String message) {
        this.message = message;
    }
    public void setStatus(Short status) {
        this.status = status;
    }
    public void setBody(String body) {
        this.body = body;
    }
    public void setRequestId(String requestId) {
        this.requestId = requestId;
    }
    public void setHeaders(Map<String, String> headers) {
        this.headers = headers;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof ErrorInfo) {
            ErrorInfo t = (ErrorInfo) other;
            return (
              Objects.equals(code, t.code) && 
              
              Objects.equals(sourceKind, t.sourceKind) && 
              
              Objects.equals(message, t.message) && 
              
              Objects.equals(status, t.status) && 
              
              Objects.equals(body, t.body) && 
              
              Objects.equals(requestId, t.requestId) && 
              
              Objects.equals(headers, t.headers)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(code, sourceKind, message, status, body, requestId, headers);
    }
}


