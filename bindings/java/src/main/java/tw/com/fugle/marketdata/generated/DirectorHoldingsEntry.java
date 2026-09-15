package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Director holdings disclosed for a single month (`date` is YYYY-MM)
 */
public class DirectorHoldingsEntry {
    private String date;
    private List<DirectorHolding> directors;

    public DirectorHoldingsEntry(
        String date, 
        List<DirectorHolding> directors
    ) {
        
        this.date = date;
        
        this.directors = directors;
    }
    
    public String date() {
        return this.date;
    }
    
    public List<DirectorHolding> directors() {
        return this.directors;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setDirectors(List<DirectorHolding> directors) {
        this.directors = directors;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof DirectorHoldingsEntry) {
            DirectorHoldingsEntry t = (DirectorHoldingsEntry) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(directors, t.directors)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, directors);
    }
}


