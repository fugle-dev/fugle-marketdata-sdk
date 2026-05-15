package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Total trading statistics
 */
public class TotalStats {
    private Double tradeValue;
    private Long tradeVolume;
    private Long tradeVolumeAtBid;
    private Long tradeVolumeAtAsk;
    private Long transaction;
    private Long time;

    public TotalStats(
        Double tradeValue, 
        Long tradeVolume, 
        Long tradeVolumeAtBid, 
        Long tradeVolumeAtAsk, 
        Long transaction, 
        Long time
    ) {
        
        this.tradeValue = tradeValue;
        
        this.tradeVolume = tradeVolume;
        
        this.tradeVolumeAtBid = tradeVolumeAtBid;
        
        this.tradeVolumeAtAsk = tradeVolumeAtAsk;
        
        this.transaction = transaction;
        
        this.time = time;
    }
    
    public Double tradeValue() {
        return this.tradeValue;
    }
    
    public Long tradeVolume() {
        return this.tradeVolume;
    }
    
    public Long tradeVolumeAtBid() {
        return this.tradeVolumeAtBid;
    }
    
    public Long tradeVolumeAtAsk() {
        return this.tradeVolumeAtAsk;
    }
    
    public Long transaction() {
        return this.transaction;
    }
    
    public Long time() {
        return this.time;
    }
    public void setTradeValue(Double tradeValue) {
        this.tradeValue = tradeValue;
    }
    public void setTradeVolume(Long tradeVolume) {
        this.tradeVolume = tradeVolume;
    }
    public void setTradeVolumeAtBid(Long tradeVolumeAtBid) {
        this.tradeVolumeAtBid = tradeVolumeAtBid;
    }
    public void setTradeVolumeAtAsk(Long tradeVolumeAtAsk) {
        this.tradeVolumeAtAsk = tradeVolumeAtAsk;
    }
    public void setTransaction(Long transaction) {
        this.transaction = transaction;
    }
    public void setTime(Long time) {
        this.time = time;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof TotalStats) {
            TotalStats t = (TotalStats) other;
            return (
              Objects.equals(tradeValue, t.tradeValue) && 
              
              Objects.equals(tradeVolume, t.tradeVolume) && 
              
              Objects.equals(tradeVolumeAtBid, t.tradeVolumeAtBid) && 
              
              Objects.equals(tradeVolumeAtAsk, t.tradeVolumeAtAsk) && 
              
              Objects.equals(transaction, t.transaction) && 
              
              Objects.equals(time, t.time)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(tradeValue, tradeVolume, tradeVolumeAtBid, tradeVolumeAtAsk, transaction, time);
    }
}


