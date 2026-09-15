package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * TDCC shareholder distribution on a single date
 */
public class TdccDistributionEntry {
    private String date;
    private List<TdccDistributionLevel> distributions;

    public TdccDistributionEntry(
        String date, 
        List<TdccDistributionLevel> distributions
    ) {
        
        this.date = date;
        
        this.distributions = distributions;
    }
    
    public String date() {
        return this.date;
    }
    
    public List<TdccDistributionLevel> distributions() {
        return this.distributions;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setDistributions(List<TdccDistributionLevel> distributions) {
        this.distributions = distributions;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof TdccDistributionEntry) {
            TdccDistributionEntry t = (TdccDistributionEntry) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(distributions, t.distributions)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, distributions);
    }
}


