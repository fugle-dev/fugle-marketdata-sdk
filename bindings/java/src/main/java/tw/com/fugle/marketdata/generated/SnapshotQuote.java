package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Single snapshot quote
 */
public class SnapshotQuote {
    private String dataType;
    private String symbol;
    private String name;
    private Double openPrice;
    private Double highPrice;
    private Double lowPrice;
    private Double closePrice;
    private Double change;
    private Double changePercent;
    private Long tradeVolume;
    private Double tradeValue;
    private Long lastUpdated;

    public SnapshotQuote(
        String dataType, 
        String symbol, 
        String name, 
        Double openPrice, 
        Double highPrice, 
        Double lowPrice, 
        Double closePrice, 
        Double change, 
        Double changePercent, 
        Long tradeVolume, 
        Double tradeValue, 
        Long lastUpdated
    ) {
        
        this.dataType = dataType;
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.openPrice = openPrice;
        
        this.highPrice = highPrice;
        
        this.lowPrice = lowPrice;
        
        this.closePrice = closePrice;
        
        this.change = change;
        
        this.changePercent = changePercent;
        
        this.tradeVolume = tradeVolume;
        
        this.tradeValue = tradeValue;
        
        this.lastUpdated = lastUpdated;
    }
    
    public String dataType() {
        return this.dataType;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String name() {
        return this.name;
    }
    
    public Double openPrice() {
        return this.openPrice;
    }
    
    public Double highPrice() {
        return this.highPrice;
    }
    
    public Double lowPrice() {
        return this.lowPrice;
    }
    
    public Double closePrice() {
        return this.closePrice;
    }
    
    public Double change() {
        return this.change;
    }
    
    public Double changePercent() {
        return this.changePercent;
    }
    
    public Long tradeVolume() {
        return this.tradeVolume;
    }
    
    public Double tradeValue() {
        return this.tradeValue;
    }
    
    public Long lastUpdated() {
        return this.lastUpdated;
    }
    public void setDataType(String dataType) {
        this.dataType = dataType;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setOpenPrice(Double openPrice) {
        this.openPrice = openPrice;
    }
    public void setHighPrice(Double highPrice) {
        this.highPrice = highPrice;
    }
    public void setLowPrice(Double lowPrice) {
        this.lowPrice = lowPrice;
    }
    public void setClosePrice(Double closePrice) {
        this.closePrice = closePrice;
    }
    public void setChange(Double change) {
        this.change = change;
    }
    public void setChangePercent(Double changePercent) {
        this.changePercent = changePercent;
    }
    public void setTradeVolume(Long tradeVolume) {
        this.tradeVolume = tradeVolume;
    }
    public void setTradeValue(Double tradeValue) {
        this.tradeValue = tradeValue;
    }
    public void setLastUpdated(Long lastUpdated) {
        this.lastUpdated = lastUpdated;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof SnapshotQuote) {
            SnapshotQuote t = (SnapshotQuote) other;
            return (
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(openPrice, t.openPrice) && 
              
              Objects.equals(highPrice, t.highPrice) && 
              
              Objects.equals(lowPrice, t.lowPrice) && 
              
              Objects.equals(closePrice, t.closePrice) && 
              
              Objects.equals(change, t.change) && 
              
              Objects.equals(changePercent, t.changePercent) && 
              
              Objects.equals(tradeVolume, t.tradeVolume) && 
              
              Objects.equals(tradeValue, t.tradeValue) && 
              
              Objects.equals(lastUpdated, t.lastUpdated)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(dataType, symbol, name, openPrice, highPrice, lowPrice, closePrice, change, changePercent, tradeVolume, tradeValue, lastUpdated);
    }
}


