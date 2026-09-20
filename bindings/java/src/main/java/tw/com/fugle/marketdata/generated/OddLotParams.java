package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * The odd-lot session flag for `stock/intraday/ticker`, `quote` and `volumes`.
 */
public class OddLotParams {
    /**
     * `true` asks for the intraday odd-lot session (`type=oddlot`).
     */
    private Boolean oddLot;

    public OddLotParams(
        Boolean oddLot
    ) {
        
        this.oddLot = oddLot;
    }
    
    public Boolean oddLot() {
        return this.oddLot;
    }
    public void setOddLot(Boolean oddLot) {
        this.oddLot = oddLot;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof OddLotParams) {
            OddLotParams t = (OddLotParams) other;
            return (
              Objects.equals(oddLot, t.oddLot)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(oddLot);
    }
}


