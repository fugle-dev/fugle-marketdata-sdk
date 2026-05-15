package tw.com.fugle.marketdata.generated;



/**
 * Error type for UniFFI bindings
 *
 * Maps to MarketDataError in the UDL file. Each variant becomes an exception
 * in the target language with the error message preserved.
 *
 * Note: This is a FLAT enum per UniFFI constraints - no nested error types.
 */
public class MarketDataException extends Exception {
    private MarketDataException(String message) {
      super(message); 
    }

    
    public static class NetworkException extends MarketDataException {
      
      String msg;
      public NetworkException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class AuthException extends MarketDataException {
      
      String msg;
      public AuthException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class RateLimitException extends MarketDataException {
      
      String msg;
      public RateLimitException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class InvalidSymbol extends MarketDataException {
      
      String msg;
      public InvalidSymbol(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class ParseException extends MarketDataException {
      
      String msg;
      public ParseException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class TimeoutException extends MarketDataException {
      
      String msg;
      public TimeoutException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class WebSocketException extends MarketDataException {
      
      String msg;
      public WebSocketException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class ClientClosed extends MarketDataException {
      public ClientClosed() {
        super(new StringBuilder()
        .toString());
        }

      
      
      
    }
    
    public static class ConfigException extends MarketDataException {
      
      String msg;
      public ConfigException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class ApiException extends MarketDataException {
      
      String msg;
      public ApiException(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
    
    public static class Other extends MarketDataException {
      
      String msg;
      public Other(String msg) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        
        .toString());
        this.msg = msg;
        }

      public String msg() {
        return this.msg;
      }
      
      
      
    }
     
}

