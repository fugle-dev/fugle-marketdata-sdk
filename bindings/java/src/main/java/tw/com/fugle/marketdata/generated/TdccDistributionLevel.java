package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * One holding-size bracket of the TDCC shareholder distribution
 */
public class TdccDistributionLevel {
    private String range;
    private Long holders;
    private Double shares;
    private Double proportion;

    public TdccDistributionLevel(
        String range, 
        Long holders, 
        Double shares, 
        Double proportion
    ) {
        
        this.range = range;
        
        this.holders = holders;
        
        this.shares = shares;
        
        this.proportion = proportion;
    }
    
    public String range() {
        return this.range;
    }
    
    public Long holders() {
        return this.holders;
    }
    
    public Double shares() {
        return this.shares;
    }
    
    public Double proportion() {
        return this.proportion;
    }
    public void setRange(String range) {
        this.range = range;
    }
    public void setHolders(Long holders) {
        this.holders = holders;
    }
    public void setShares(Double shares) {
        this.shares = shares;
    }
    public void setProportion(Double proportion) {
        this.proportion = proportion;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof TdccDistributionLevel) {
            TdccDistributionLevel t = (TdccDistributionLevel) other;
            return (
              Objects.equals(range, t.range) && 
              
              Objects.equals(holders, t.holders) && 
              
              Objects.equals(shares, t.shares) && 
              
              Objects.equals(proportion, t.proportion)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(range, holders, shares, proportion);
    }
}


