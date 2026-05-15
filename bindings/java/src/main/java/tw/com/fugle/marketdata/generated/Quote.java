package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Real-time stock quote
 */
public class Quote {
    private String date;
    private String dataType;
    private String exchange;
    private String market;
    private String symbol;
    private String name;
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
    private List<PriceLevel> bids;
    private List<PriceLevel> asks;
    private TotalStats total;
    private TradeInfo lastTrade;
    private TradeInfo lastTrial;
    private TradingHalt tradingHalt;
    private Boolean isLimitDownPrice;
    private Boolean isLimitUpPrice;
    private Boolean isLimitDownBid;
    private Boolean isLimitUpBid;
    private Boolean isLimitDownAsk;
    private Boolean isLimitUpAsk;
    private Boolean isLimitDownHalt;
    private Boolean isLimitUpHalt;
    private Boolean isTrial;
    private Boolean isDelayedOpen;
    private Boolean isDelayedClose;
    private Boolean isContinuous;
    private Boolean isOpen;
    private Boolean isClose;
    private Long lastUpdated;

    public Quote(
        String date, 
        String dataType, 
        String exchange, 
        String market, 
        String symbol, 
        String name, 
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
        List<PriceLevel> bids, 
        List<PriceLevel> asks, 
        TotalStats total, 
        TradeInfo lastTrade, 
        TradeInfo lastTrial, 
        TradingHalt tradingHalt, 
        Boolean isLimitDownPrice, 
        Boolean isLimitUpPrice, 
        Boolean isLimitDownBid, 
        Boolean isLimitUpBid, 
        Boolean isLimitDownAsk, 
        Boolean isLimitUpAsk, 
        Boolean isLimitDownHalt, 
        Boolean isLimitUpHalt, 
        Boolean isTrial, 
        Boolean isDelayedOpen, 
        Boolean isDelayedClose, 
        Boolean isContinuous, 
        Boolean isOpen, 
        Boolean isClose, 
        Long lastUpdated
    ) {
        
        this.date = date;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.symbol = symbol;
        
        this.name = name;
        
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
        
        this.lastTrial = lastTrial;
        
        this.tradingHalt = tradingHalt;
        
        this.isLimitDownPrice = isLimitDownPrice;
        
        this.isLimitUpPrice = isLimitUpPrice;
        
        this.isLimitDownBid = isLimitDownBid;
        
        this.isLimitUpBid = isLimitUpBid;
        
        this.isLimitDownAsk = isLimitDownAsk;
        
        this.isLimitUpAsk = isLimitUpAsk;
        
        this.isLimitDownHalt = isLimitDownHalt;
        
        this.isLimitUpHalt = isLimitUpHalt;
        
        this.isTrial = isTrial;
        
        this.isDelayedOpen = isDelayedOpen;
        
        this.isDelayedClose = isDelayedClose;
        
        this.isContinuous = isContinuous;
        
        this.isOpen = isOpen;
        
        this.isClose = isClose;
        
        this.lastUpdated = lastUpdated;
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
    
    public List<PriceLevel> bids() {
        return this.bids;
    }
    
    public List<PriceLevel> asks() {
        return this.asks;
    }
    
    public TotalStats total() {
        return this.total;
    }
    
    public TradeInfo lastTrade() {
        return this.lastTrade;
    }
    
    public TradeInfo lastTrial() {
        return this.lastTrial;
    }
    
    public TradingHalt tradingHalt() {
        return this.tradingHalt;
    }
    
    public Boolean isLimitDownPrice() {
        return this.isLimitDownPrice;
    }
    
    public Boolean isLimitUpPrice() {
        return this.isLimitUpPrice;
    }
    
    public Boolean isLimitDownBid() {
        return this.isLimitDownBid;
    }
    
    public Boolean isLimitUpBid() {
        return this.isLimitUpBid;
    }
    
    public Boolean isLimitDownAsk() {
        return this.isLimitDownAsk;
    }
    
    public Boolean isLimitUpAsk() {
        return this.isLimitUpAsk;
    }
    
    public Boolean isLimitDownHalt() {
        return this.isLimitDownHalt;
    }
    
    public Boolean isLimitUpHalt() {
        return this.isLimitUpHalt;
    }
    
    public Boolean isTrial() {
        return this.isTrial;
    }
    
    public Boolean isDelayedOpen() {
        return this.isDelayedOpen;
    }
    
    public Boolean isDelayedClose() {
        return this.isDelayedClose;
    }
    
    public Boolean isContinuous() {
        return this.isContinuous;
    }
    
    public Boolean isOpen() {
        return this.isOpen;
    }
    
    public Boolean isClose() {
        return this.isClose;
    }
    
    public Long lastUpdated() {
        return this.lastUpdated;
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
    public void setBids(List<PriceLevel> bids) {
        this.bids = bids;
    }
    public void setAsks(List<PriceLevel> asks) {
        this.asks = asks;
    }
    public void setTotal(TotalStats total) {
        this.total = total;
    }
    public void setLastTrade(TradeInfo lastTrade) {
        this.lastTrade = lastTrade;
    }
    public void setLastTrial(TradeInfo lastTrial) {
        this.lastTrial = lastTrial;
    }
    public void setTradingHalt(TradingHalt tradingHalt) {
        this.tradingHalt = tradingHalt;
    }
    public void setIsLimitDownPrice(Boolean isLimitDownPrice) {
        this.isLimitDownPrice = isLimitDownPrice;
    }
    public void setIsLimitUpPrice(Boolean isLimitUpPrice) {
        this.isLimitUpPrice = isLimitUpPrice;
    }
    public void setIsLimitDownBid(Boolean isLimitDownBid) {
        this.isLimitDownBid = isLimitDownBid;
    }
    public void setIsLimitUpBid(Boolean isLimitUpBid) {
        this.isLimitUpBid = isLimitUpBid;
    }
    public void setIsLimitDownAsk(Boolean isLimitDownAsk) {
        this.isLimitDownAsk = isLimitDownAsk;
    }
    public void setIsLimitUpAsk(Boolean isLimitUpAsk) {
        this.isLimitUpAsk = isLimitUpAsk;
    }
    public void setIsLimitDownHalt(Boolean isLimitDownHalt) {
        this.isLimitDownHalt = isLimitDownHalt;
    }
    public void setIsLimitUpHalt(Boolean isLimitUpHalt) {
        this.isLimitUpHalt = isLimitUpHalt;
    }
    public void setIsTrial(Boolean isTrial) {
        this.isTrial = isTrial;
    }
    public void setIsDelayedOpen(Boolean isDelayedOpen) {
        this.isDelayedOpen = isDelayedOpen;
    }
    public void setIsDelayedClose(Boolean isDelayedClose) {
        this.isDelayedClose = isDelayedClose;
    }
    public void setIsContinuous(Boolean isContinuous) {
        this.isContinuous = isContinuous;
    }
    public void setIsOpen(Boolean isOpen) {
        this.isOpen = isOpen;
    }
    public void setIsClose(Boolean isClose) {
        this.isClose = isClose;
    }
    public void setLastUpdated(Long lastUpdated) {
        this.lastUpdated = lastUpdated;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof Quote) {
            Quote t = (Quote) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
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
              
              Objects.equals(lastTrial, t.lastTrial) && 
              
              Objects.equals(tradingHalt, t.tradingHalt) && 
              
              Objects.equals(isLimitDownPrice, t.isLimitDownPrice) && 
              
              Objects.equals(isLimitUpPrice, t.isLimitUpPrice) && 
              
              Objects.equals(isLimitDownBid, t.isLimitDownBid) && 
              
              Objects.equals(isLimitUpBid, t.isLimitUpBid) && 
              
              Objects.equals(isLimitDownAsk, t.isLimitDownAsk) && 
              
              Objects.equals(isLimitUpAsk, t.isLimitUpAsk) && 
              
              Objects.equals(isLimitDownHalt, t.isLimitDownHalt) && 
              
              Objects.equals(isLimitUpHalt, t.isLimitUpHalt) && 
              
              Objects.equals(isTrial, t.isTrial) && 
              
              Objects.equals(isDelayedOpen, t.isDelayedOpen) && 
              
              Objects.equals(isDelayedClose, t.isDelayedClose) && 
              
              Objects.equals(isContinuous, t.isContinuous) && 
              
              Objects.equals(isOpen, t.isOpen) && 
              
              Objects.equals(isClose, t.isClose) && 
              
              Objects.equals(lastUpdated, t.lastUpdated)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, dataType, exchange, market, symbol, name, openPrice, openTime, highPrice, highTime, lowPrice, lowTime, closePrice, closeTime, lastPrice, lastSize, avgPrice, change, changePercent, amplitude, bids, asks, total, lastTrade, lastTrial, tradingHalt, isLimitDownPrice, isLimitUpPrice, isLimitDownBid, isLimitUpBid, isLimitDownAsk, isLimitUpAsk, isLimitDownHalt, isLimitUpHalt, isTrial, isDelayedOpen, isDelayedClose, isContinuous, isOpen, isClose, lastUpdated);
    }
}


