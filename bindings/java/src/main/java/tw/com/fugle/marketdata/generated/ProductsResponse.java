package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.nio.ByteBuffer;
import java.util.Objects;
/**
 * FutOpt products response
 */
public class ProductsResponse {
    private String date;
    private String productType;
    private String session;
    private String contractType;
    private String status;
    private List<Product> data;

    public ProductsResponse(
        String date, 
        String productType, 
        String session, 
        String contractType, 
        String status, 
        List<Product> data
    ) {
        
        this.date = date;
        
        this.productType = productType;
        
        this.session = session;
        
        this.contractType = contractType;
        
        this.status = status;
        
        this.data = data;
    }
    
    public String date() {
        return this.date;
    }
    
    public String productType() {
        return this.productType;
    }
    
    public String session() {
        return this.session;
    }
    
    public String contractType() {
        return this.contractType;
    }
    
    public String status() {
        return this.status;
    }
    
    public List<Product> data() {
        return this.data;
    }
    public void setDate(String date) {
        this.date = date;
    }
    public void setProductType(String productType) {
        this.productType = productType;
    }
    public void setSession(String session) {
        this.session = session;
    }
    public void setContractType(String contractType) {
        this.contractType = contractType;
    }
    public void setStatus(String status) {
        this.status = status;
    }
    public void setData(List<Product> data) {
        this.data = data;
    }

    
    
    @Override
    public boolean equals(Object other) {
        if (other instanceof ProductsResponse) {
            ProductsResponse t = (ProductsResponse) other;
            return (
              Objects.equals(date, t.date) && 
              
              Objects.equals(productType, t.productType) && 
              
              Objects.equals(session, t.session) && 
              
              Objects.equals(contractType, t.contractType) && 
              
              Objects.equals(status, t.status) && 
              
              Objects.equals(data, t.data)
              
            );
        };
        return false;
    }

    @Override
    public int hashCode() {
        return Objects.hash(date, productType, session, contractType, status, data);
    }
}


