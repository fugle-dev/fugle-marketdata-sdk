package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt quote
 */
public class FutOptQuote {
    private String date;
    private String contractType;
    private String exchange;
    private String symbol;
    private String name;
    private Double previousClose;
    private Double openPrice;
    private Long openTime;
    private Double highPrice;
    private Long highTime;
    private Double lowPrice;
    private Long lowTime;
    private Double closePrice;
    private Long closeTime;
    private Double lastPrice;
    private Long lastSize;
    private Double avgPrice;
    private Double change;
    private Double changePercent;
    private Double amplitude;
    private List<FutOptPriceLevel> bids;
    private List<FutOptPriceLevel> asks;
    private FutOptTotalStats total;
    private FutOptLastTrade lastTrade;
    private Long lastUpdated;

    public FutOptQuote(
        String date, 
        String contractType, 
        String exchange, 
        String symbol, 
        String name, 
        Double previousClose, 
        Double openPrice, 
        Long openTime, 
        Double highPrice, 
        Long highTime, 
        Double lowPrice, 
        Long lowTime, 
        Double closePrice, 
        Long closeTime, 
        Double lastPrice, 
        Long lastSize, 
        Double avgPrice, 
        Double change, 
        Double changePercent, 
        Double amplitude, 
        List<FutOptPriceLevel> bids, 
        List<FutOptPriceLevel> asks, 
        FutOptTotalStats total, 
        FutOptLastTrade lastTrade, 
        Long lastUpdated
    ) {
        
        this.date = date;
        
        this.contractType = contractType;
        
        this.exchange = exchange;
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.previousClose = previousClose;
        
        this.openPrice = openPrice;
        
        this.openTime = openTime;
        
        this.highPrice = highPrice;
        
        this.highTime = highTime;
        
        this.lowPrice = lowPrice;
        
        this.lowTime = lowTime;
        
        this.closePrice = closePrice;
        
        this.closeTime = closeTime;
        
        this.lastPrice = lastPrice;
        
        this.lastSize = lastSize;
        
        this.avgPrice = avgPrice;
        
        this.change = change;
        
        this.changePercent = changePercent;
        
        this.amplitude = amplitude;
        
        this.bids = bids;
        
        this.asks = asks;
        
        this.total = total;
        
        this.lastTrade = lastTrade;
        
        this.lastUpdated = lastUpdated;
    }
    
    public String date() {
        return this.date;
    }
    
    public String contractType() {
        return this.contractType;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public String symbol() {
        return this.symbol;
    }
    
    public String name() {
        return this.name;
    }
    
    public Double previousClose() {
        return this.previousClose;
    }
    
    public Double openPrice() {
        return this.openPrice;
    }
    
    public Long openTime() {
        return this.openTime;
    }
    
    public Double highPrice() {
        return this.highPrice;
    }
    
    public Long highTime() {
        return this.highTime;
    }
    
    public Double lowPrice() {
        return this.lowPrice;
    }
    
    public Long lowTime() {
        return this.lowTime;
    }
    
    public Double closePrice() {
        return this.closePrice;
    }
    
    public Long closeTime() {
        return this.closeTime;
    }
    
    public Double lastPrice() {
        return this.lastPrice;
    }
    
    public Long lastSize() {
        return this.lastSize;
    }
    
    public Double avgPrice() {
        return this.avgPrice;
    }
    
    public Double change() {
        return this.change;
    }
    
    public Double changePercent() {
        return this.changePercent;
    }
    
    public Double amplitude() {
        return this.amplitude;
    }
    
    public List<FutOptPriceLevel> bids() {
        return this.bids;
    }
    
    public List<FutOptPriceLevel> asks() {
        return this.asks;
    }
    
    public FutOptTotalStats total() {
        return this.total;
    }
    
    public FutOptLastTrade lastTrade() {
        return this.lastTrade;
    }
    
    public Long lastUpdated() {
        return this.lastUpdated;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setContractType(String contractType) {
        this.contractType = contractType;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setSymbol(String symbol) {
        this.symbol = symbol;
    }
    public void setName(String name) {
        this.name = name;
    }
    public void setPreviousClose(Double previousClose) {
        this.previousClose = previousClose;
    }
    public void setOpenPrice(Double openPrice) {
        this.openPrice = openPrice;
    }
    public void setOpenTime(Long openTime) {
        this.openTime = openTime;
    }
    public void setHighPrice(Double highPrice) {
        this.highPrice = highPrice;
    }
    public void setHighTime(Long highTime) {
        this.highTime = highTime;
    }
    public void setLowPrice(Double lowPrice) {
        this.lowPrice = lowPrice;
    }
    public void setLowTime(Long lowTime) {
        this.lowTime = lowTime;
    }
    public void setClosePrice(Double closePrice) {
        this.closePrice = closePrice;
    }
    public void setCloseTime(Long closeTime) {
        this.closeTime = closeTime;
    }
    public void setLastPrice(Double lastPrice) {
        this.lastPrice = lastPrice;
    }
    public void setLastSize(Long lastSize) {
        this.lastSize = lastSize;
    }
    public void setAvgPrice(Double avgPrice) {
        this.avgPrice = avgPrice;
    }
    public void setChange(Double change) {
        this.change = change;
    }
    public void setChangePercent(Double changePercent) {
        this.changePercent = changePercent;
    }
    public void setAmplitude(Double amplitude) {
        this.amplitude = amplitude;
    }
    public void setBids(List<FutOptPriceLevel> bids) {
        this.bids = bids;
    }
    public void setAsks(List<FutOptPriceLevel> asks) {
        this.asks = asks;
    }
    public void setTotal(FutOptTotalStats total) {
        this.total = total;
    }
    public void setLastTrade(FutOptLastTrade lastTrade) {
        this.lastTrade = lastTrade;
    }
    public void setLastUpdated(Long lastUpdated) {
        this.lastUpdated = lastUpdated;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptQuote) {
            FutOptQuote t = (FutOptQuote) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(contractType, t.contractType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(previousClose, t.previousClose) && 
              
              Objects.equals(openPrice, t.openPrice) && 
              
              Objects.equals(openTime, t.openTime) && 
              
              Objects.equals(highPrice, t.highPrice) && 
              
              Objects.equals(highTime, t.highTime) && 
              
              Objects.equals(lowPrice, t.lowPrice) && 
              
              Objects.equals(lowTime, t.lowTime) && 
              
              Objects.equals(closePrice, t.closePrice) && 
              
              Objects.equals(closeTime, t.closeTime) && 
              
              Objects.equals(lastPrice, t.lastPrice) && 
              
              Objects.equals(lastSize, t.lastSize) && 
              
              Objects.equals(avgPrice, t.avgPrice) && 
              
              Objects.equals(change, t.change) && 
              
              Objects.equals(changePercent, t.changePercent) && 
              
              Objects.equals(amplitude, t.amplitude) && 
              
              Objects.equals(bids, t.bids) && 
              
              Objects.equals(asks, t.asks) && 
              
              Objects.equals(total, t.total) && 
              
              Objects.equals(lastTrade, t.lastTrade) && 
              
              Objects.equals(lastUpdated, t.lastUpdated)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, contractType, exchange, symbol, name, previousClose, openPrice, openTime, highPrice, highTime, lowPrice, lowTime, closePrice, closeTime, lastPrice, lastSize, avgPrice, change, changePercent, amplitude, bids, asks, total, lastTrade, lastUpdated);
    }
}


