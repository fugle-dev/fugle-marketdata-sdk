package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `futopt/historical/daily`.
 */
public class FutOptDailyParams {
    /**
     * `YYYY-MM-DD`.
     */
    private String date;
    /**
     * `true` asks for the after-hours session (`session=afterhours`).
     */
    private Boolean afterHours;

    public FutOptDailyParams(
        String date, 
        Boolean afterHours
    ) {
        
        this.date = date;
        
        this.afterHours = afterHours;
    }
    
    public String date() {
        return this.date;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptDailyParams) {
            FutOptDailyParams t = (FutOptDailyParams) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(afterHours, t.afterHours)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, afterHours);
    }
}


