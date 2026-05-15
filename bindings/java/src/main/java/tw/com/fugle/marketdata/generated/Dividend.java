package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Dividend entry
 */
public class Dividend {
    private String symbol;
    private String name;
    private String exDividendDate;
    private String paymentDate;
    private Double cashDividend;
    private Double stockDividend;
    private String dividendYear;

    public Dividend(
        String symbol, 
        String name, 
        String exDividendDate, 
        String paymentDate, 
        Double cashDividend, 
        Double stockDividend, 
        String dividendYear
    ) {
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.exDividendDate = exDividendDate;
        
        this.paymentDate = paymentDate;
        
        this.cashDividend = cashDividend;
        
        this.stockDividend = stockDividend;
        
        this.dividendYear = dividendYear;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String name() {
        return this.name;
    }
    
    public String exDividendDate() {
        return this.exDividendDate;
    }
    
    public String paymentDate() {
        return this.paymentDate;
    }
    
    public Double cashDividend() {
        return this.cashDividend;
    }
    
    public Double stockDividend() {
        return this.stockDividend;
    }
    
    public String dividendYear() {
        return this.dividendYear;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setExDividendDate(String exDividendDate) {
        this.exDividendDate = exDividendDate;
    }
    public void setPaymentDate(String paymentDate) {
        this.paymentDate = paymentDate;
    }
    public void setCashDividend(Double cashDividend) {
        this.cashDividend = cashDividend;
    }
    public void setStockDividend(Double stockDividend) {
        this.stockDividend = stockDividend;
    }
    public void setDividendYear(String dividendYear) {
        this.dividendYear = dividendYear;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof Dividend) {
            Dividend t = (Dividend) other;
            return (
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(exDividendDate, t.exDividendDate) && 
              
              Objects.equals(paymentDate, t.paymentDate) && 
              
              Objects.equals(cashDividend, t.cashDividend) && 
              
              Objects.equals(stockDividend, t.stockDividend) && 
              
              Objects.equals(dividendYear, t.dividendYear)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(symbol, name, exDividendDate, paymentDate, cashDividend, stockDividend, dividendYear);
    }
}


