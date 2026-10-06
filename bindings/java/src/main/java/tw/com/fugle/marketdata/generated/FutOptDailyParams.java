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
    /**
     * One contract month only: `YYYYMM`, `YYYYMMWn` / `YYYYMMFn`, a futures
     * spread `YYYYMM/YYYYMM`, or (futures only) `1!` / `2!` / `3!`. Unset
     * returns every contract month.
     */
    private String contractMonth;

    public FutOptDailyParams(
        String date, 
        Boolean afterHours, 
        String contractMonth
    ) {
        
        this.date = date;
        
        this.afterHours = afterHours;
        
        this.contractMonth = contractMonth;
    }
    
    public String date() {
        return this.date;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    
    public String contractMonth() {
        return this.contractMonth;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }
    public void setContractMonth(String contractMonth) {
        this.contractMonth = contractMonth;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptDailyParams) {
            FutOptDailyParams t = (FutOptDailyParams) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(afterHours, t.afterHours) && 
              
              Objects.equals(contractMonth, t.contractMonth)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, afterHours, contractMonth);
    }
}


