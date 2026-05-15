package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Optional TLS customization exposed to foreign languages.
 *
 * When all fields are default the SDK uses the OS trust store
 * (loaded by `rustls-native-certs`). Provide `root_cert_pem` to pin
 * an additional CA, or set `accept_invalid_certs` to disable all
 * verification (dev/testing only — exposes MITM risk).
 */
public class TlsConfigRecord {
    /**
     * PEM-encoded additional root CA bytes. Appended to the OS trust
     * store; chains signed by either this CA or any OS-trusted root
     * are accepted.
     */
    private byte[] rootCertPem;
    /**
     * Disable ALL TLS verification (chain + hostname + expiry).
     * Equivalent to `curl -k` / `wscat --no-check`. Do not use in
     * production.
     */
    private Boolean acceptInvalidCerts;

    public TlsConfigRecord(
        byte[] rootCertPem, 
        Boolean acceptInvalidCerts
    ) {
        
        this.rootCertPem = rootCertPem;
        
        this.acceptInvalidCerts = acceptInvalidCerts;
    }
    
    public byte[] rootCertPem() {
        return this.rootCertPem;
    }
    
    public Boolean acceptInvalidCerts() {
        return this.acceptInvalidCerts;
    }
    public void setRootCertPem(byte[] rootCertPem) {
        this.rootCertPem = rootCertPem;
    }
    public void setAcceptInvalidCerts(Boolean acceptInvalidCerts) {
        this.acceptInvalidCerts = acceptInvalidCerts;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof TlsConfigRecord) {
            TlsConfigRecord t = (TlsConfigRecord) other;
            return (
              Objects.equals(rootCertPem, t.rootCertPem) && 
              
              Objects.equals(acceptInvalidCerts, t.acceptInvalidCerts)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(rootCertPem, acceptInvalidCerts);
    }
}


