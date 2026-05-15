package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Listing applicants response
 */
public class ListingApplicantsResponse {
    private String dataType;
    private String exchange;
    private String market;
    private List<ListingApplicant> data;

    public ListingApplicantsResponse(
        String dataType, 
        String exchange, 
        String market, 
        List<ListingApplicant> data
    ) {
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.data = data;
    }
    
    public String dataType() {
        return this.dataType;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public String market() {
        return this.market;
    }
    
    public List<ListingApplicant> data() {
        return this.data;
    }
    public void setDataType(String dataType) {
        this.dataType = dataType;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setMarket(String market) {
        this.market = market;
    }
    public void setData(List<ListingApplicant> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof ListingApplicantsResponse) {
            ListingApplicantsResponse t = (ListingApplicantsResponse) other;
            return (
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(dataType, exchange, market, data);
    }
}


