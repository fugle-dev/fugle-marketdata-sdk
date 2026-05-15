package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Capital change entry
 */
public class CapitalChange {
    private String symbol;
    private String name;
    private String date;
    private Double previousCapital;
    private Double currentCapital;
    private String changeType;
    private String reason;

    public CapitalChange(
        String symbol, 
        String name, 
        String date, 
        Double previousCapital, 
        Double currentCapital, 
        String changeType, 
        String reason
    ) {
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.date = date;
        
        this.previousCapital = previousCapital;
        
        this.currentCapital = currentCapital;
        
        this.changeType = changeType;
        
        this.reason = reason;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String name() {
        return this.name;
    }
    
    public String date() {
        return this.date;
    }
    
    public Double previousCapital() {
        return this.previousCapital;
    }
    
    public Double currentCapital() {
        return this.currentCapital;
    }
    
    public String changeType() {
        return this.changeType;
    }
    
    public String reason() {
        return this.reason;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setPreviousCapital(Double previousCapital) {
        this.previousCapital = previousCapital;
    }
    public void setCurrentCapital(Double currentCapital) {
        this.currentCapital = currentCapital;
    }
    public void setChangeType(String changeType) {
        this.changeType = changeType;
    }
    public void setReason(String reason) {
        this.reason = reason;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof CapitalChange) {
            CapitalChange t = (CapitalChange) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(date, t.date) && 
              
              Objects.equals(previousCapital, t.previousCapital) && 
              
              Objects.equals(currentCapital, t.currentCapital) && 
              
              Objects.equals(changeType, t.changeType) && 
              
              Objects.equals(reason, t.reason)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, name, date, previousCapital, currentCapital, changeType, reason);
    }
}


