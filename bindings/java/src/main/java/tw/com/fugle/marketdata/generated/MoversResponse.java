package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Movers response
 */
public class MoversResponse {
    private String date;
    private String time;
    private String market;
    private List<Mover> data;

    public MoversResponse(
        String date, 
        String time, 
        String market, 
        List<Mover> data
    ) {
        
        this.date = date;
        
        this.time = time;
        
        this.market = market;
        
        this.data = data;
    }
    
    public String date() {
        return this.date;
    }
    
    public String time() {
        return this.time;
    }
    
    public String market() {
        return this.market;
    }
    
    public List<Mover> data() {
        return this.data;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setTime(String time) {
        this.time = time;
    }
    public void setMarket(String market) {
        this.market = market;
    }
    public void setData(List<Mover> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof MoversResponse) {
            MoversResponse t = (MoversResponse) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(time, t.time) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, time, market, data);
    }
}


