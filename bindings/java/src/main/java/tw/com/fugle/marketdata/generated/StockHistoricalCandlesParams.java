package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `stock/historical/candles`.
 */
public class StockHistoricalCandlesParams {
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
    /**
     * Comma-separated field names, `open,high,low,close,volume`.
     */
    private String fields;
    /**
     * `asc` or `desc`.
     */
    private String sort;
    /**
     * Adjusted prices.
     */
    private Boolean adjusted;

    public StockHistoricalCandlesParams(
        String from, 
        String to, 
        String timeframe, 
        String fields, 
        String sort, 
        Boolean adjusted
    ) {
        
        this.from = from;
        
        this.to = to;
        
        this.timeframe = timeframe;
        
        this.fields = fields;
        
        this.sort = sort;
        
        this.adjusted = adjusted;
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
    
    public String fields() {
        return this.fields;
    }
    
    public String sort() {
        return this.sort;
    }
    
    public Boolean adjusted() {
        return this.adjusted;
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
    public void setFields(String fields) {
        this.fields = fields;
    }
    public void setSort(String sort) {
        this.sort = sort;
    }
    public void setAdjusted(Boolean adjusted) {
        this.adjusted = adjusted;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StockHistoricalCandlesParams) {
            StockHistoricalCandlesParams t = (StockHistoricalCandlesParams) other;
            return (
              Objects.equals(from, t.from) && 
              
              Objects.equals(to, t.to) && 
              
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(fields, t.fields) && 
              
              Objects.equals(sort, t.sort) && 
              
              Objects.equals(adjusted, t.adjusted)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(from, to, timeframe, fields, sort, adjusted);
    }
}


