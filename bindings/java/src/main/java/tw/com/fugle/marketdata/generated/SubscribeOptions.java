package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Session options for `subscribe` / `unsubscribe` (#202).
 *
 * Unset is the regular session, so an omitted or default record subscribes
 * as before. Each option belongs to one endpoint — `intraday_odd_lot`
 * (盤中零股) to Stock, `after_hours` (盤後) to FutOpt — and setting it on
 * the other, to any value, is 1005 `INVALID_PARAMETER`.
 */
public class SubscribeOptions {
    /**
     * FutOpt only: `true` subscribes to the after-hours session.
     */
    private Boolean afterHours;
    /**
     * Stock only: `true` subscribes to the intraday odd-lot session.
     */
    private Boolean intradayOddLot;

    public SubscribeOptions(
        Boolean afterHours, 
        Boolean intradayOddLot
    ) {
        
        this.afterHours = afterHours;
        
        this.intradayOddLot = intradayOddLot;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    
    public Boolean intradayOddLot() {
        return this.intradayOddLot;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }
    public void setIntradayOddLot(Boolean intradayOddLot) {
        this.intradayOddLot = intradayOddLot;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof SubscribeOptions) {
            SubscribeOptions t = (SubscribeOptions) other;
            return (
              Objects.equals(afterHours, t.afterHours) && 
              
              Objects.equals(intradayOddLot, t.intradayOddLot)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(afterHours, intradayOddLot);
    }
}


