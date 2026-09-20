package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Parameters for `stock/intraday/candles`.
 */
public class StockCandlesParams {
    /**
     * `1`, `5`, `10`, `15`, `30` or `60` minutes; unset takes the server
     * default.
     */
    private String timeframe;
    /**
     * `true` asks for the intraday odd-lot session (`type=oddlot`).
     */
    private Boolean oddLot;
    /**
     * `asc` or `desc`.
     */
    private String sort;

    public StockCandlesParams(
        String timeframe, 
        Boolean oddLot, 
        String sort
    ) {
        
        this.timeframe = timeframe;
        
        this.oddLot = oddLot;
        
        this.sort = sort;
    }
    
    public String timeframe() {
        return this.timeframe;
    }
    
    public Boolean oddLot() {
        return this.oddLot;
    }
    
    public String sort() {
        return this.sort;
    }
    public void setTimeframe(String timeframe) {
        this.timeframe = timeframe;
    }
    public void setOddLot(Boolean oddLot) {
        this.oddLot = oddLot;
    }
    public void setSort(String sort) {
        this.sort = sort;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StockCandlesParams) {
            StockCandlesParams t = (StockCandlesParams) other;
            return (
              Objects.equals(timeframe, t.timeframe) && 
              
              Objects.equals(oddLot, t.oddLot) && 
              
              Objects.equals(sort, t.sort)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(timeframe, oddLot, sort);
    }
}


