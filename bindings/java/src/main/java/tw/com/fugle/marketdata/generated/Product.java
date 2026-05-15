package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt product
 */
public class Product {
    private String productType;
    private String exchange;
    private String symbol;
    private String name;
    private String underlyingSymbol;
    private String contractType;
    private Long contractSize;
    private String underlyingType;
    private String statusCode;
    private String tradingCurrency;
    private Boolean quoteAcceptable;
    private Boolean canBlockTrade;
    private String startDate;
    private String expiryType;
    private Integer marketCloseGroup;
    private Integer endSession;

    public Product(
        String productType, 
        String exchange, 
        String symbol, 
        String name, 
        String underlyingSymbol, 
        String contractType, 
        Long contractSize, 
        String underlyingType, 
        String statusCode, 
        String tradingCurrency, 
        Boolean quoteAcceptable, 
        Boolean canBlockTrade, 
        String startDate, 
        String expiryType, 
        Integer marketCloseGroup, 
        Integer endSession
    ) {
        
        this.productType = productType;
        
        this.exchange = exchange;
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.underlyingSymbol = underlyingSymbol;
        
        this.contractType = contractType;
        
        this.contractSize = contractSize;
        
        this.underlyingType = underlyingType;
        
        this.statusCode = statusCode;
        
        this.tradingCurrency = tradingCurrency;
        
        this.quoteAcceptable = quoteAcceptable;
        
        this.canBlockTrade = canBlockTrade;
        
        this.startDate = startDate;
        
        this.expiryType = expiryType;
        
        this.marketCloseGroup = marketCloseGroup;
        
        this.endSession = endSession;
    }
    
    public String productType() {
        return this.productType;
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
    
    public String underlyingSymbol() {
        return this.underlyingSymbol;
    }
    
    public String contractType() {
        return this.contractType;
    }
    
    public Long contractSize() {
        return this.contractSize;
    }
    
    public String underlyingType() {
        return this.underlyingType;
    }
    
    public String statusCode() {
        return this.statusCode;
    }
    
    public String tradingCurrency() {
        return this.tradingCurrency;
    }
    
    public Boolean quoteAcceptable() {
        return this.quoteAcceptable;
    }
    
    public Boolean canBlockTrade() {
        return this.canBlockTrade;
    }
    
    public String startDate() {
        return this.startDate;
    }
    
    public String expiryType() {
        return this.expiryType;
    }
    
    public Integer marketCloseGroup() {
        return this.marketCloseGroup;
    }
    
    public Integer endSession() {
        return this.endSession;
    }
    public void setProductType(String productType) {
        this.productType = productType;
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
    public void setUnderlyingSymbol(String underlyingSymbol) {
        this.underlyingSymbol = underlyingSymbol;
    }
    public void setContractType(String contractType) {
        this.contractType = contractType;
    }
    public void setContractSize(Long contractSize) {
        this.contractSize = contractSize;
    }
    public void setUnderlyingType(String underlyingType) {
        this.underlyingType = underlyingType;
    }
    public void setStatusCode(String statusCode) {
        this.statusCode = statusCode;
    }
    public void setTradingCurrency(String tradingCurrency) {
        this.tradingCurrency = tradingCurrency;
    }
    public void setQuoteAcceptable(Boolean quoteAcceptable) {
        this.quoteAcceptable = quoteAcceptable;
    }
    public void setCanBlockTrade(Boolean canBlockTrade) {
        this.canBlockTrade = canBlockTrade;
    }
    public void setStartDate(String startDate) {
        this.startDate = startDate;
    }
    public void setExpiryType(String expiryType) {
        this.expiryType = expiryType;
    }
    public void setMarketCloseGroup(Integer marketCloseGroup) {
        this.marketCloseGroup = marketCloseGroup;
    }
    public void setEndSession(Integer endSession) {
        this.endSession = endSession;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof Product) {
            Product t = (Product) other;
            return (
              Objects.equals(productType, t.productType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(underlyingSymbol, t.underlyingSymbol) && 
              
              Objects.equals(contractType, t.contractType) && 
              
              Objects.equals(contractSize, t.contractSize) && 
              
              Objects.equals(underlyingType, t.underlyingType) && 
              
              Objects.equals(statusCode, t.statusCode) && 
              
              Objects.equals(tradingCurrency, t.tradingCurrency) && 
              
              Objects.equals(quoteAcceptable, t.quoteAcceptable) && 
              
              Objects.equals(canBlockTrade, t.canBlockTrade) && 
              
              Objects.equals(startDate, t.startDate) && 
              
              Objects.equals(expiryType, t.expiryType) && 
              
              Objects.equals(marketCloseGroup, t.marketCloseGroup) && 
              
              Objects.equals(endSession, t.endSession)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(productType, exchange, symbol, name, underlyingSymbol, contractType, contractSize, underlyingType, statusCode, tradingCurrency, quoteAcceptable, canBlockTrade, startDate, expiryType, marketCloseGroup, endSession);
    }
}


