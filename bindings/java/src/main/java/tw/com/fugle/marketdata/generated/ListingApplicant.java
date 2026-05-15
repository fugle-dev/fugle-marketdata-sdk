package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Listing applicant entry
 */
public class ListingApplicant {
    private String symbol;
    private String name;
    private String applicationDate;
    private String listingDate;
    private String status;
    private String industry;

    public ListingApplicant(
        String symbol, 
        String name, 
        String applicationDate, 
        String listingDate, 
        String status, 
        String industry
    ) {
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.applicationDate = applicationDate;
        
        this.listingDate = listingDate;
        
        this.status = status;
        
        this.industry = industry;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String name() {
        return this.name;
    }
    
    public String applicationDate() {
        return this.applicationDate;
    }
    
    public String listingDate() {
        return this.listingDate;
    }
    
    public String status() {
        return this.status;
    }
    
    public String industry() {
        return this.industry;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setApplicationDate(String applicationDate) {
        this.applicationDate = applicationDate;
    }
    public void setListingDate(String listingDate) {
        this.listingDate = listingDate;
    }
    public void setStatus(String status) {
        this.status = status;
    }
    public void setIndustry(String industry) {
        this.industry = industry;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof ListingApplicant) {
            ListingApplicant t = (ListingApplicant) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(applicationDate, t.applicationDate) && 
              
              Objects.equals(listingDate, t.listingDate) && 
              
              Objects.equals(status, t.status) && 
              
              Objects.equals(industry, t.industry)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, name, applicationDate, listingDate, status, industry);
    }
}


