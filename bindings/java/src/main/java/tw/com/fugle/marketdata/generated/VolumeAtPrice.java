package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Volume at a specific price level
 */
public class VolumeAtPrice {
    private Double price;
    private Long volume;
    private Long volumeAtBid;
    private Long volumeAtAsk;

    public VolumeAtPrice(
        Double price, 
        Long volume, 
        Long volumeAtBid, 
        Long volumeAtAsk
    ) {
        
        this.price = price;
        
        this.volume = volume;
        
        this.volumeAtBid = volumeAtBid;
        
        this.volumeAtAsk = volumeAtAsk;
    }
    
    public Double price() {
        return this.price;
    }
    
    public Long volume() {
        return this.volume;
    }
    
    public Long volumeAtBid() {
        return this.volumeAtBid;
    }
    
    public Long volumeAtAsk() {
        return this.volumeAtAsk;
    }
    public void setPrice(Double price) {
        this.price = price;
    }
    public void setVolume(Long volume) {
        this.volume = volume;
    }
    public void setVolumeAtBid(Long volumeAtBid) {
        this.volumeAtBid = volumeAtBid;
    }
    public void setVolumeAtAsk(Long volumeAtAsk) {
        this.volumeAtAsk = volumeAtAsk;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof VolumeAtPrice) {
            VolumeAtPrice t = (VolumeAtPrice) other;
            return (
              Objects.equals(price, t.price) && 
              
              Objects.equals(volume, t.volume) && 
              
              Objects.equals(volumeAtBid, t.volumeAtBid) && 
              
              Objects.equals(volumeAtAsk, t.volumeAtAsk)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(price, volume, volumeAtBid, volumeAtAsk);
    }
}


