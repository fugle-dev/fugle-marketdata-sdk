package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `futopt/intraday/candles`.
 */
public class FutOptCandlesParams {
    /**
     * `true` asks for the after-hours session (`session=afterhours`).
     */
    private Boolean afterHours;
    /**
     * `1`, `5`, `10`, `15`, `30` or `60` minutes.
     */
    private String timeframe;

    public FutOptCandlesParams(
        Boolean afterHours, 
        String timeframe
    ) {
        
        this.afterHours = afterHours;
        
        this.timeframe = timeframe;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    
    public String timeframe() {
        return this.timeframe;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptCandlesParams) {
            FutOptCandlesParams t = (FutOptCandlesParams) other;
            return (
              Objects.equals(afterHours, t.afterHours) && 
              
              Objects.equals(timeframe, t.timeframe)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(afterHours, timeframe);
    }
}


