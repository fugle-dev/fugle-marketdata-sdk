package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * The after-hours session flag for `futopt/intraday/ticker`, `quote` and
 * `volumes`.
 */
public class AfterHoursParams {
    /**
     * `true` asks for the after-hours session (`session=afterhours`);
     * unset or `false` is the regular session.
     */
    private Boolean afterHours;

    public AfterHoursParams(
        Boolean afterHours
    ) {
        
        this.afterHours = afterHours;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof AfterHoursParams) {
            AfterHoursParams t = (AfterHoursParams) other;
            return (
              Objects.equals(afterHours, t.afterHours)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(afterHours);
    }
}


