package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `stock/snapshot/quotes` and `actives`.
 */
public class SnapshotParams {
    /**
     * `type`: `ALL`, `ALLBUT0999` or `COMMONSTOCK`.
     */
    private String typeFilter;

    public SnapshotParams(
        String typeFilter
    ) {
        
        this.typeFilter = typeFilter;
    }
    
    public String typeFilter() {
        return this.typeFilter;
    }
    public void setTypeFilter(String typeFilter) {
        this.typeFilter = typeFilter;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof SnapshotParams) {
            SnapshotParams t = (SnapshotParams) other;
            return (
              Objects.equals(typeFilter, t.typeFilter)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(typeFilter);
    }
}


