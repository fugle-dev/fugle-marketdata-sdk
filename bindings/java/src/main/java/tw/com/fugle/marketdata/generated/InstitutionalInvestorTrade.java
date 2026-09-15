package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Buy / sell / net shares traded by one class of institutional investor.
 * Fields are `None` when the source has no figure for that day.
 */
public class InstitutionalInvestorTrade {
    private Double buy;
    private Double sell;
    private Double net;

    public InstitutionalInvestorTrade(
        Double buy, 
        Double sell, 
        Double net
    ) {
        
        this.buy = buy;
        
        this.sell = sell;
        
        this.net = net;
    }
    
    public Double buy() {
        return this.buy;
    }
    
    public Double sell() {
        return this.sell;
    }
    
    public Double net() {
        return this.net;
    }
    public void setBuy(Double buy) {
        this.buy = buy;
    }
    public void setSell(Double sell) {
        this.sell = sell;
    }
    public void setNet(Double net) {
        this.net = net;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof InstitutionalInvestorTrade) {
            InstitutionalInvestorTrade t = (InstitutionalInvestorTrade) other;
            return (
              Objects.equals(buy, t.buy) && 
              
              Objects.equals(sell, t.sell) && 
              
              Objects.equals(net, t.net)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(buy, sell, net);
    }
}


