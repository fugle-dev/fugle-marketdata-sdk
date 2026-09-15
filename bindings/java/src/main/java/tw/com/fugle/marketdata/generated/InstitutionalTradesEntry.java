package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Institutional investor trading on a single date
 */
public class InstitutionalTradesEntry {
    private String date;
    private InstitutionalInvestorTrade foreign;
    private InstitutionalInvestorTrade trust;
    private InstitutionalInvestorTrade dealer;
    private Double total;

    public InstitutionalTradesEntry(
        String date, 
        InstitutionalInvestorTrade foreign, 
        InstitutionalInvestorTrade trust, 
        InstitutionalInvestorTrade dealer, 
        Double total
    ) {
        
        this.date = date;
        
        this.foreign = foreign;
        
        this.trust = trust;
        
        this.dealer = dealer;
        
        this.total = total;
    }
    
    public String date() {
        return this.date;
    }
    
    public InstitutionalInvestorTrade foreign() {
        return this.foreign;
    }
    
    public InstitutionalInvestorTrade trust() {
        return this.trust;
    }
    
    public InstitutionalInvestorTrade dealer() {
        return this.dealer;
    }
    
    public Double total() {
        return this.total;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setForeign(InstitutionalInvestorTrade foreign) {
        this.foreign = foreign;
    }
    public void setTrust(InstitutionalInvestorTrade trust) {
        this.trust = trust;
    }
    public void setDealer(InstitutionalInvestorTrade dealer) {
        this.dealer = dealer;
    }
    public void setTotal(Double total) {
        this.total = total;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof InstitutionalTradesEntry) {
            InstitutionalTradesEntry t = (InstitutionalTradesEntry) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(foreign, t.foreign) && 
              
              Objects.equals(trust, t.trust) && 
              
              Objects.equals(dealer, t.dealer) && 
              
              Objects.equals(total, t.total)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, foreign, trust, dealer, total);
    }
}


