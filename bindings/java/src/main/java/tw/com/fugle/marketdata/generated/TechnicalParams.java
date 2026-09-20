package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * The date range for the `stock/technical` endpoints; the periods are the
 * method's arguments.
 */
public class TechnicalParams {
    /**
     * Start date, `YYYY-MM-DD`.
     */
    private String from;
    /**
     * End date, `YYYY-MM-DD`.
     */
    private String to;
    /**
     * `D`, `W`, `M`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
     */
    private String timeframe;

    public TechnicalParams(
        String from, 
        String to, 
        String timeframe
    ) {
        
        this.from = from;
        
        this.to = to;
        
        this.timeframe = timeframe;
    }
    
    public String from() {
        return this.from;
    }
    
    public String to() {
        return this.to;
    }
    
    public String timeframe() {
        return this.timeframe;
    }
    public void setFrom(String from) {
        this.from = from;
    }
    public void setTo(String to) {
        this.to = to;
    }
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof TechnicalParams) {
            TechnicalParams t = (TechnicalParams) other;
            return (
              Objects.equals(from, t.from) && 
              
              Objects.equals(to, t.to) && 
              
              Objects.equals(timeframe, t.timeframe)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(from, to, timeframe);
    }
}


