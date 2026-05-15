package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt ticker
 */
public class FutOptTicker {
    private String date;
    private String contractType;
    private String exchange;
    private String symbol;
    private String name;
    private Double referencePrice;
    private String startDate;
    private String endDate;
    private String settlementDate;
    private String contractSubType;
    private Boolean isDynamicBanding;
    private Integer flowGroup;

    public FutOptTicker(
        String date, 
        String contractType, 
        String exchange, 
        String symbol, 
        String name, 
        Double referencePrice, 
        String startDate, 
        String endDate, 
        String settlementDate, 
        String contractSubType, 
        Boolean isDynamicBanding, 
        Integer flowGroup
    ) {
        
        this.date = date;
        
        this.contractType = contractType;
        
        this.exchange = exchange;
        
        this.symbol = symbol;
        
        this.name = name;
        
        this.referencePrice = referencePrice;
        
        this.startDate = startDate;
        
        this.endDate = endDate;
        
        this.settlementDate = settlementDate;
        
        this.contractSubType = contractSubType;
        
        this.isDynamicBanding = isDynamicBanding;
        
        this.flowGroup = flowGroup;
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
    
    public Double referencePrice() {
        return this.referencePrice;
    }
    
    public String startDate() {
        return this.startDate;
    }
    
    public String endDate() {
        return this.endDate;
    }
    
    public String settlementDate() {
        return this.settlementDate;
    }
    
    public String contractSubType() {
        return this.contractSubType;
    }
    
    public Boolean isDynamicBanding() {
        return this.isDynamicBanding;
    }
    
    public Integer flowGroup() {
        return this.flowGroup;
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
    public void setReferencePrice(Double referencePrice) {
        this.referencePrice = referencePrice;
    }
    public void setStartDate(String startDate) {
        this.startDate = startDate;
    }
    public void setEndDate(String endDate) {
        this.endDate = endDate;
    }
    public void setSettlementDate(String settlementDate) {
        this.settlementDate = settlementDate;
    }
    public void setContractSubType(String contractSubType) {
        this.contractSubType = contractSubType;
    }
    public void setIsDynamicBanding(Boolean isDynamicBanding) {
        this.isDynamicBanding = isDynamicBanding;
    }
    public void setFlowGroup(Integer flowGroup) {
        this.flowGroup = flowGroup;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof FutOptTicker) {
            FutOptTicker t = (FutOptTicker) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(contractType, t.contractType) && 
              
              Objects.equals(exchange, t.exchange) && 
              
              Objects.equals(symbol, t.symbol) && 
              
              Objects.equals(name, t.name) && 
              
              Objects.equals(referencePrice, t.referencePrice) && 
              
              Objects.equals(startDate, t.startDate) && 
              
              Objects.equals(endDate, t.endDate) && 
              
              Objects.equals(settlementDate, t.settlementDate) && 
              
              Objects.equals(contractSubType, t.contractSubType) && 
              
              Objects.equals(isDynamicBanding, t.isDynamicBanding) && 
              
              Objects.equals(flowGroup, t.flowGroup)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, contractType, exchange, symbol, name, referencePrice, startDate, endDate, settlementDate, contractSubType, isDynamicBanding, flowGroup);
    }
}


