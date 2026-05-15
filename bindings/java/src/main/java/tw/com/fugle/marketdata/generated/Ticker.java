package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * Stock ticker info
 */
public class Ticker {
    private String date;
    private String dataType;
    private String exchange;
    private String market;
    private String symbol;
    private String name;
    private String nameEn;
    private String industry;
    private String securityType;
    private Double referencePrice;
    private Double limitUpPrice;
    private Double limitDownPrice;
    private Double previousClose;
    private Boolean canDayTrade;
    private Boolean canBuyDayTrade;
    private Boolean canBelowFlatMarginShortSell;
    private Boolean canBelowFlatSblShortSell;
    private Boolean isAttention;
    private Boolean isDisposition;
    private Boolean isUnusuallyRecommended;
    private Boolean isSpecificAbnormally;
    private Boolean isNewlyCompiled;
    private Integer matchingInterval;
    private String securityStatus;
    private Integer boardLot;
    private String tradingCurrency;
    private Double exercisePrice;
    private Long exercisedVolume;
    private Long cancelledVolume;
    private Long remainingVolume;
    private Double exerciseRatio;
    private Double capPrice;
    private Double floorPrice;
    private String maturityDate;
    private String openTime;
    private String closeTime;

    public Ticker(
        String date, 
        String dataType, 
        String exchange, 
        String market, 
        String symbol, 
        String name, 
        String nameEn, 
        String industry, 
        String securityType, 
        Double referencePrice, 
        Double limitUpPrice, 
        Double limitDownPrice, 
        Double previousClose, 
        Boolean canDayTrade, 
        Boolean canBuyDayTrade, 
        Boolean canBelowFlatMarginShortSell, 
        Boolean canBelowFlatSblShortSell, 
        Boolean isAttention, 
        Boolean isDisposition, 
        Boolean isUnusuallyRecommended, 
        Boolean isSpecificAbnormally, 
        Boolean isNewlyCompiled, 
        Integer matchingInterval, 
        String securityStatus, 
        Integer boardLot, 
        String tradingCurrency, 
        Double exercisePrice, 
        Long exercisedVolume, 
        Long cancelledVolume, 
        Long remainingVolume, 
        Double exerciseRatio, 
        Double capPrice, 
        Double floorPrice, 
        String maturityDate, 
        String openTime, 
        String closeTime
    ) {
        
        this.date = date;
        
        this.dataType = dataType;
        
        this.exchange = exchange;
        
        this.market = market;
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.nameEn = nameEn;
        
        this.industry = industry;
        
        this.securityType = securityType;
        
        this.referencePrice = referencePrice;
        
        this.limitUpPrice = limitUpPrice;
        
        this.limitDownPrice = limitDownPrice;
        
        this.previousClose = previousClose;
        
        this.canDayTrade = canDayTrade;
        
        this.canBuyDayTrade = canBuyDayTrade;
        
        this.canBelowFlatMarginShortSell = canBelowFlatMarginShortSell;
        
        this.canBelowFlatSblShortSell = canBelowFlatSblShortSell;
        
        this.isAttention = isAttention;
        
        this.isDisposition = isDisposition;
        
        this.isUnusuallyRecommended = isUnusuallyRecommended;
        
        this.isSpecificAbnormally = isSpecificAbnormally;
        
        this.isNewlyCompiled = isNewlyCompiled;
        
        this.matchingInterval = matchingInterval;
        
        this.securityStatus = securityStatus;
        
        this.boardLot = boardLot;
        
        this.tradingCurrency = tradingCurrency;
        
        this.exercisePrice = exercisePrice;
        
        this.exercisedVolume = exercisedVolume;
        
        this.cancelledVolume = cancelledVolume;
        
        this.remainingVolume = remainingVolume;
        
        this.exerciseRatio = exerciseRatio;
        
        this.capPrice = capPrice;
        
        this.floorPrice = floorPrice;
        
        this.maturityDate = maturityDate;
        
        this.openTime = openTime;
        
        this.closeTime = closeTime;
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
    
    public String nameEn() {
        return this.nameEn;
    }
    
    public String industry() {
        return this.industry;
    }
    
    public String securityType() {
        return this.securityType;
    }
    
    public Double referencePrice() {
        return this.referencePrice;
    }
    
    public Double limitUpPrice() {
        return this.limitUpPrice;
    }
    
    public Double limitDownPrice() {
        return this.limitDownPrice;
    }
    
    public Double previousClose() {
        return this.previousClose;
    }
    
    public Boolean canDayTrade() {
        return this.canDayTrade;
    }
    
    public Boolean canBuyDayTrade() {
        return this.canBuyDayTrade;
    }
    
    public Boolean canBelowFlatMarginShortSell() {
        return this.canBelowFlatMarginShortSell;
    }
    
    public Boolean canBelowFlatSblShortSell() {
        return this.canBelowFlatSblShortSell;
    }
    
    public Boolean isAttention() {
        return this.isAttention;
    }
    
    public Boolean isDisposition() {
        return this.isDisposition;
    }
    
    public Boolean isUnusuallyRecommended() {
        return this.isUnusuallyRecommended;
    }
    
    public Boolean isSpecificAbnormally() {
        return this.isSpecificAbnormally;
    }
    
    public Boolean isNewlyCompiled() {
        return this.isNewlyCompiled;
    }
    
    public Integer matchingInterval() {
        return this.matchingInterval;
    }
    
    public String securityStatus() {
        return this.securityStatus;
    }
    
    public Integer boardLot() {
        return this.boardLot;
    }
    
    public String tradingCurrency() {
        return this.tradingCurrency;
    }
    
    public Double exercisePrice() {
        return this.exercisePrice;
    }
    
    public Long exercisedVolume() {
        return this.exercisedVolume;
    }
    
    public Long cancelledVolume() {
        return this.cancelledVolume;
    }
    
    public Long remainingVolume() {
        return this.remainingVolume;
    }
    
    public Double exerciseRatio() {
        return this.exerciseRatio;
    }
    
    public Double capPrice() {
        return this.capPrice;
    }
    
    public Double floorPrice() {
        return this.floorPrice;
    }
    
    public String maturityDate() {
        return this.maturityDate;
    }
    
    public String openTime() {
        return this.openTime;
    }
    
    public String closeTime() {
        return this.closeTime;
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
    public void setNameEn(String nameEn) {
        this.nameEn = nameEn;
    }
    public void setIndustry(String industry) {
        this.industry = industry;
    }
    public void setSecurityType(String securityType) {
        this.securityType = securityType;
    }
    public void setReferencePrice(Double referencePrice) {
        this.referencePrice = referencePrice;
    }
    public void setLimitUpPrice(Double limitUpPrice) {
        this.limitUpPrice = limitUpPrice;
    }
    public void setLimitDownPrice(Double limitDownPrice) {
        this.limitDownPrice = limitDownPrice;
    }
    public void setPreviousClose(Double previousClose) {
        this.previousClose = previousClose;
    }
    public void setCanDayTrade(Boolean canDayTrade) {
        this.canDayTrade = canDayTrade;
    }
    public void setCanBuyDayTrade(Boolean canBuyDayTrade) {
        this.canBuyDayTrade = canBuyDayTrade;
    }
    public void setCanBelowFlatMarginShortSell(Boolean canBelowFlatMarginShortSell) {
        this.canBelowFlatMarginShortSell = canBelowFlatMarginShortSell;
    }
    public void setCanBelowFlatSblShortSell(Boolean canBelowFlatSblShortSell) {
        this.canBelowFlatSblShortSell = canBelowFlatSblShortSell;
    }
    public void setIsAttention(Boolean isAttention) {
        this.isAttention = isAttention;
    }
    public void setIsDisposition(Boolean isDisposition) {
        this.isDisposition = isDisposition;
    }
    public void setIsUnusuallyRecommended(Boolean isUnusuallyRecommended) {
        this.isUnusuallyRecommended = isUnusuallyRecommended;
    }
    public void setIsSpecificAbnormally(Boolean isSpecificAbnormally) {
        this.isSpecificAbnormally = isSpecificAbnormally;
    }
    public void setIsNewlyCompiled(Boolean isNewlyCompiled) {
        this.isNewlyCompiled = isNewlyCompiled;
    }
    public void setMatchingInterval(Integer matchingInterval) {
        this.matchingInterval = matchingInterval;
    }
    public void setSecurityStatus(String securityStatus) {
        this.securityStatus = securityStatus;
    }
    public void setBoardLot(Integer boardLot) {
        this.boardLot = boardLot;
    }
    public void setTradingCurrency(String tradingCurrency) {
        this.tradingCurrency = tradingCurrency;
    }
    public void setExercisePrice(Double exercisePrice) {
        this.exercisePrice = exercisePrice;
    }
    public void setExercisedVolume(Long exercisedVolume) {
        this.exercisedVolume = exercisedVolume;
    }
    public void setCancelledVolume(Long cancelledVolume) {
        this.cancelledVolume = cancelledVolume;
    }
    public void setRemainingVolume(Long remainingVolume) {
        this.remainingVolume = remainingVolume;
    }
    public void setExerciseRatio(Double exerciseRatio) {
        this.exerciseRatio = exerciseRatio;
    }
    public void setCapPrice(Double capPrice) {
        this.capPrice = capPrice;
    }
    public void setFloorPrice(Double floorPrice) {
        this.floorPrice = floorPrice;
    }
    public void setMaturityDate(String maturityDate) {
        this.maturityDate = maturityDate;
    }
    public void setOpenTime(String openTime) {
        this.openTime = openTime;
    }
    public void setCloseTime(String closeTime) {
        this.closeTime = closeTime;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof Ticker) {
            Ticker t = (Ticker) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(dataType, t.dataType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(market, t.market) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(nameEn, t.nameEn) && 
              
              Objects.equals(industry, t.industry) && 
              
              Objects.equals(securityType, t.securityType) && 
              
              Objects.equals(referencePrice, t.referencePrice) && 
              
              Objects.equals(limitUpPrice, t.limitUpPrice) && 
              
              Objects.equals(limitDownPrice, t.limitDownPrice) && 
              
              Objects.equals(previousClose, t.previousClose) && 
              
              Objects.equals(canDayTrade, t.canDayTrade) && 
              
              Objects.equals(canBuyDayTrade, t.canBuyDayTrade) && 
              
              Objects.equals(canBelowFlatMarginShortSell, t.canBelowFlatMarginShortSell) && 
              
              Objects.equals(canBelowFlatSblShortSell, t.canBelowFlatSblShortSell) && 
              
              Objects.equals(isAttention, t.isAttention) && 
              
              Objects.equals(isDisposition, t.isDisposition) && 
              
              Objects.equals(isUnusuallyRecommended, t.isUnusuallyRecommended) && 
              
              Objects.equals(isSpecificAbnormally, t.isSpecificAbnormally) && 
              
              Objects.equals(isNewlyCompiled, t.isNewlyCompiled) && 
              
              Objects.equals(matchingInterval, t.matchingInterval) && 
              
              Objects.equals(securityStatus, t.securityStatus) && 
              
              Objects.equals(boardLot, t.boardLot) && 
              
              Objects.equals(tradingCurrency, t.tradingCurrency) && 
              
              Objects.equals(exercisePrice, t.exercisePrice) && 
              
              Objects.equals(exercisedVolume, t.exercisedVolume) && 
              
              Objects.equals(cancelledVolume, t.cancelledVolume) && 
              
              Objects.equals(remainingVolume, t.remainingVolume) && 
              
              Objects.equals(exerciseRatio, t.exerciseRatio) && 
              
              Objects.equals(capPrice, t.capPrice) && 
              
              Objects.equals(floorPrice, t.floorPrice) && 
              
              Objects.equals(maturityDate, t.maturityDate) && 
              
              Objects.equals(openTime, t.openTime) && 
              
              Objects.equals(closeTime, t.closeTime)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, dataType, exchange, market, symbol, name, nameEn, industry, securityType, referencePrice, limitUpPrice, limitDownPrice, previousClose, canDayTrade, canBuyDayTrade, canBelowFlatMarginShortSell, canBelowFlatSblShortSell, isAttention, isDisposition, isUnusuallyRecommended, isSpecificAbnormally, isNewlyCompiled, matchingInterval, securityStatus, boardLot, tradingCurrency, exercisePrice, exercisedVolume, cancelledVolume, remainingVolume, exerciseRatio, capPrice, floorPrice, maturityDate, openTime, closeTime);
    }
}


