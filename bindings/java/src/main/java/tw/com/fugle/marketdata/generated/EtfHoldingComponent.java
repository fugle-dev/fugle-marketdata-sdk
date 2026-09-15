package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * One constituent of an ETF's holdings on a given date
 */
public class EtfHoldingComponent {
    private String symbol;
    private String name;
    private Double quantity;
    private Double weight;
    /**
     * Absent on the first date in a series — nothing to compare against.
     */
    private Double quantityChange;
    private Double weightChange;

    public EtfHoldingComponent(
        String symbol, 
        String name, 
        Double quantity, 
        Double weight, 
        Double quantityChange, 
        Double weightChange
    ) {
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.quantity = quantity;
        
        this.weight = weight;
        
        this.quantityChange = quantityChange;
        
        this.weightChange = weightChange;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String name() {
        return this.name;
    }
    
    public Double quantity() {
        return this.quantity;
    }
    
    public Double weight() {
        return this.weight;
    }
    
    public Double quantityChange() {
        return this.quantityChange;
    }
    
    public Double weightChange() {
        return this.weightChange;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setQuantity(Double quantity) {
        this.quantity = quantity;
    }
    public void setWeight(Double weight) {
        this.weight = weight;
    }
    public void setQuantityChange(Double quantityChange) {
        this.quantityChange = quantityChange;
    }
    public void setWeightChange(Double weightChange) {
        this.weightChange = weightChange;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof EtfHoldingComponent) {
            EtfHoldingComponent t = (EtfHoldingComponent) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(quantity, t.quantity) && 
              
              Objects.equals(weight, t.weight) && 
              
              Objects.equals(quantityChange, t.quantityChange) && 
              
              Objects.equals(weightChange, t.weightChange)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, name, quantity, weight, quantityChange, weightChange);
    }
}


