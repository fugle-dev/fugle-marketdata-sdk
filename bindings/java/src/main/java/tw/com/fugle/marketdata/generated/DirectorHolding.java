package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * One director's or supervisor's disclosed holdings
 */
public class DirectorHolding {
    private Long order;
    private String title;
    private String name;
    private Double electedShares;
    private Double heldShares;
    private Double pledgedShares;
    private Double pledgeRatio;
    private Double relatedHeldShares;
    private Double relatedPledgedShares;
    private Double relatedPledgeRatio;

    public DirectorHolding(
        Long order, 
        String title, 
        String name, 
        Double electedShares, 
        Double heldShares, 
        Double pledgedShares, 
        Double pledgeRatio, 
        Double relatedHeldShares, 
        Double relatedPledgedShares, 
        Double relatedPledgeRatio
    ) {
        
        this.order = order;
        
        this.title = title;
        
        this.name = name;
        
        this.electedShares = electedShares;
        
        this.heldShares = heldShares;
        
        this.pledgedShares = pledgedShares;
        
        this.pledgeRatio = pledgeRatio;
        
        this.relatedHeldShares = relatedHeldShares;
        
        this.relatedPledgedShares = relatedPledgedShares;
        
        this.relatedPledgeRatio = relatedPledgeRatio;
    }
    
    public Long order() {
        return this.order;
    }
    
    public String title() {
        return this.title;
    }
    
    public String name() {
        return this.name;
    }
    
    public Double electedShares() {
        return this.electedShares;
    }
    
    public Double heldShares() {
        return this.heldShares;
    }
    
    public Double pledgedShares() {
        return this.pledgedShares;
    }
    
    public Double pledgeRatio() {
        return this.pledgeRatio;
    }
    
    public Double relatedHeldShares() {
        return this.relatedHeldShares;
    }
    
    public Double relatedPledgedShares() {
        return this.relatedPledgedShares;
    }
    
    public Double relatedPledgeRatio() {
        return this.relatedPledgeRatio;
    }
    public void setOrder(Long order) {
        this.order = order;
    }
    public void setTitle(String title) {
        this.title = title;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setElectedShares(Double electedShares) {
        this.electedShares = electedShares;
    }
    public void setHeldShares(Double heldShares) {
        this.heldShares = heldShares;
    }
    public void setPledgedShares(Double pledgedShares) {
        this.pledgedShares = pledgedShares;
    }
    public void setPledgeRatio(Double pledgeRatio) {
        this.pledgeRatio = pledgeRatio;
    }
    public void setRelatedHeldShares(Double relatedHeldShares) {
        this.relatedHeldShares = relatedHeldShares;
    }
    public void setRelatedPledgedShares(Double relatedPledgedShares) {
        this.relatedPledgedShares = relatedPledgedShares;
    }
    public void setRelatedPledgeRatio(Double relatedPledgeRatio) {
        this.relatedPledgeRatio = relatedPledgeRatio;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof DirectorHolding) {
            DirectorHolding t = (DirectorHolding) other;
            return (
              Objects.equals(order, t.order) && 
              
              Objects.equals(title, t.title) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(electedShares, t.electedShares) && 
              
              Objects.equals(heldShares, t.heldShares) && 
              
              Objects.equals(pledgedShares, t.pledgedShares) && 
              
              Objects.equals(pledgeRatio, t.pledgeRatio) && 
              
              Objects.equals(relatedHeldShares, t.relatedHeldShares) && 
              
              Objects.equals(relatedPledgedShares, t.relatedPledgedShares) && 
              
              Objects.equals(relatedPledgeRatio, t.relatedPledgeRatio)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(order, title, name, electedShares, heldShares, pledgedShares, pledgeRatio, relatedHeldShares, relatedPledgedShares, relatedPledgeRatio);
    }
}


