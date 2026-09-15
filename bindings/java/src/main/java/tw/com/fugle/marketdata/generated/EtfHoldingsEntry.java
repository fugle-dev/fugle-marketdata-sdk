package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Holdings disclosed on a single date
 */
public class EtfHoldingsEntry {
    private String date;
    private List<EtfHoldingComponent> components;

    public EtfHoldingsEntry(
        String date, 
        List<EtfHoldingComponent> components
    ) {
        
        this.date = date;
        
        this.components = components;
    }
    
    public String date() {
        return this.date;
    }
    
    public List<EtfHoldingComponent> components() {
        return this.components;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setComponents(List<EtfHoldingComponent> components) {
        this.components = components;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof EtfHoldingsEntry) {
            EtfHoldingsEntry t = (EtfHoldingsEntry) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(components, t.components)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, components);
    }
}


