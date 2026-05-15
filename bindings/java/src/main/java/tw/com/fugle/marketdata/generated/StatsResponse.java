package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Historical stats response
 */
public class StatsResponse {
    private String date;
    private String dataType;
    private String exchange;
    private String market;
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
    private Double previousClose;
    private Double week52High;
    private Double week52Low;

    public StatsResponse(
        String date, 
        String dataType, 
        String exchange, 
        String market, 
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
        Double previousClose, 
        Double week52High, 
        Double week52Low
    ) {
        
        this.date = date;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
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
        
        this.previousClose = previousClose;
        
        this.week52High = week52High;
        
        this.week52Low = week52Low;
    }
    
    public String date() {
        return this.date;
    }
    
    public String dataType() {
        return this.dataType;
    }
    
    public String exchange() {
        return this.exchange;
    }
    
    public String market() {
        return this.market;
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
    
    public Double previousClose() {
        return this.previousClose;
    }
    
    public Double week52High() {
        return this.week52High;
    }
    
    public Double week52Low() {
        return this.week52Low;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setDataType(String dataType) {
        this.dataType = dataType;
    }
    public void setExchange(String exchange) {
        this.exchange = exchange;
    }
    public void setMarket(String market) {
        this.market = market;
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
    public void setPreviousClose(Double previousClose) {
        this.previousClose = previousClose;
    }
    public void setWeek52High(Double week52High) {
        this.week52High = week52High;
    }
    public void setWeek52Low(Double week52Low) {
        this.week52Low = week52Low;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof StatsResponse) {
            StatsResponse t = (StatsResponse) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
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
              
              Objects.equals(previousClose, t.previousClose) && 
              
              Objects.equals(week52High, t.week52High) && 
              
              Objects.equals(week52Low, t.week52Low)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, dataType, exchange, market, symbol, name, openPrice, highPrice, lowPrice, closePrice, change, changePercent, tradeVolume, tradeValue, previousClose, week52High, week52Low);
    }
}


