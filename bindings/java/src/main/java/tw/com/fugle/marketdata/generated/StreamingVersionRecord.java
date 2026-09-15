package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Per-product streaming version selection.
 *
 * UniFFI has no way to express core's one-enum-per-product typing across
 * C#/Go/Java/C++ at once, so this carries optional strings and validates
 * them — the same shape the official SDK's version map has.
 */
public class StreamingVersionRecord {
    /**
     * Stock streaming version. Only "v1.0" is served. None means latest.
     */
    private String stock;
    /**
     * FutOpt streaming version: "v1.0" or "v1.1". None means latest (v1.1).
     *
     * v1.1 adds trial-matching (試撮) frames on trades / books — check the
     * frame's `isTrial` before acting on a price.
     */
    private String futopt;

    public StreamingVersionRecord(
        String stock, 
        String futopt
    ) {
        
        this.stock = stock;
        
        this.futopt = futopt;
    }
    
    public String stock() {
        return this.stock;
    }
    
    public String futopt() {
        return this.futopt;
    }
    public void setStock(String stock) {
        this.stock = stock;
    }
    public void setFutopt(String futopt) {
        this.futopt = futopt;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StreamingVersionRecord) {
            StreamingVersionRecord t = (StreamingVersionRecord) other;
            return (
              Objects.equals(stock, t.stock) && 
              
              Objects.equals(futopt, t.futopt)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(stock, futopt);
    }
}


