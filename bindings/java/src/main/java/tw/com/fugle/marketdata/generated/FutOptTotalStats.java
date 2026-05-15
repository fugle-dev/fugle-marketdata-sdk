package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt total stats
 */
public class FutOptTotalStats {
    private Long tradeVolume;
    private Long totalBidMatch;
    private Long totalAskMatch;

    public FutOptTotalStats(
        Long tradeVolume, 
        Long totalBidMatch, 
        Long totalAskMatch
    ) {
        
        this.tradeVolume = tradeVolume;
        
        this.totalBidMatch = totalBidMatch;
        
        this.totalAskMatch = totalAskMatch;
    }
    
    public Long tradeVolume() {
        return this.tradeVolume;
    }
    
    public Long totalBidMatch() {
        return this.totalBidMatch;
    }
    
    public Long totalAskMatch() {
        return this.totalAskMatch;
    }
    public void setTradeVolume(Long tradeVolume) {
        this.tradeVolume = tradeVolume;
    }
    public void setTotalBidMatch(Long totalBidMatch) {
        this.totalBidMatch = totalBidMatch;
    }
    public void setTotalAskMatch(Long totalAskMatch) {
        this.totalAskMatch = totalAskMatch;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptTotalStats) {
            FutOptTotalStats t = (FutOptTotalStats) other;
            return (
              Objects.equals(tradeVolume, t.tradeVolume) && 
              
              Objects.equals(totalBidMatch, t.totalBidMatch) && 
              
              Objects.equals(totalAskMatch, t.totalAskMatch)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(tradeVolume, totalBidMatch, totalAskMatch);
    }
}


