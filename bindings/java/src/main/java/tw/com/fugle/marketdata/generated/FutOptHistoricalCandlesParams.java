package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `futopt/historical/candles`.
 *
 * `strike_price` is `f64`, and 0 is a strike like any other: unlike the
 * config records, an unset field is `None`, not 0.
 */
public class FutOptHistoricalCandlesParams {
    /**
     * Start date, `YYYY-MM-DD`.
     */
    private String from;
    /**
     * End date, `YYYY-MM-DD`.
     */
    private String to;
    /**
     * `YYYYMM`, or a continuous contract: `1!` (the server default), `2!`,
     * `3!`.
     */
    private String contractMonth;
    /**
     * Comma-separated field names.
     */
    private String fields;
    /**
     * `D`, or `1`, `5`, `10`, `15`, `30`, `60` minutes.
     */
    private String timeframe;
    /**
     * `asc` or `desc`.
     */
    private String sort;
    /**
     * Options only.
     */
    private Double strikePrice;
    /**
     * Options only: `C` or `P`.
     */
    private String callPut;
    /**
     * `true` asks for the after-hours session (`session=afterhours`).
     */
    private Boolean afterHours;

    public FutOptHistoricalCandlesParams(
        String from, 
        String to, 
        String contractMonth, 
        String fields, 
        String timeframe, 
        String sort, 
        Double strikePrice, 
        String callPut, 
        Boolean afterHours
    ) {
        
        this.from = from;
        
        this.to = to;
        
        this.contractMonth = contractMonth;
        
        this.fields = fields;
        
        this.timeframe = timeframe;
        
        this.sort = sort;
        
        this.strikePrice = strikePrice;
        
        this.callPut = callPut;
        
        this.afterHours = afterHours;
    }
    
    public String from() {
        return this.from;
    }
    
    public String to() {
        return this.to;
    }
    
    public String contractMonth() {
        return this.contractMonth;
    }
    
    public String fields() {
        return this.fields;
    }
    
    public String timeframe() {
        return this.timeframe;
    }
    
    public String sort() {
        return this.sort;
    }
    
    public Double strikePrice() {
        return this.strikePrice;
    }
    
    public String callPut() {
        return this.callPut;
    }
    
    public Boolean afterHours() {
        return this.afterHours;
    }
    public void setFrom(String from) {
        this.from = from;
    }
    public void setTo(String to) {
        this.to = to;
    }
    public void setContractMonth(String contractMonth) {
        this.contractMonth = contractMonth;
    }
    public void setFields(String fields) {
        this.fields = fields;
    }
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }
    public void setSort(String sort) {
        this.sort = sort;
    }
    public void setStrikePrice(Double strikePrice) {
        this.strikePrice = strikePrice;
    }
    public void setCallPut(String callPut) {
        this.callPut = callPut;
    }
    public void setAfterHours(Boolean afterHours) {
        this.afterHours = afterHours;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptHistoricalCandlesParams) {
            FutOptHistoricalCandlesParams t = (FutOptHistoricalCandlesParams) other;
            return (
              Objects.equals(from, t.from) && 
              
              Objects.equals(to, t.to) && 
              
              Objects.equals(contractMonth, t.contractMonth) && 
              
              Objects.equals(fields, t.fields) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(sort, t.sort) && 
              
              Objects.equals(strikePrice, t.strikePrice) && 
              
              Objects.equals(callPut, t.callPut) && 
              
              Objects.equals(afterHours, t.afterHours)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(from, to, contractMonth, fields, timeframe, sort, strikePrice, callPut, afterHours);
    }
}


