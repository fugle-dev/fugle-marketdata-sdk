package tw.com.fugle.marketdata.generated;



/**
 * Error type for UniFFI bindings
 *
 * Maps to MarketDataError in the UDL file. Each variant becomes an exception
 * in the target language with the error message preserved, plus an `info`
 * field carrying the unified [`ErrorInfo`].
 *
 * Note: This is a FLAT enum per UniFFI constraints - no nested error types.
 */
public class MarketDataException extends Exception {
    private MarketDataException(String message) {
      super(message); 
    }

    
    public static class NetworkException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public NetworkException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class AuthException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public AuthException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class RateLimitException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public RateLimitException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class InvalidSymbol extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public InvalidSymbol(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class ParseException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public ParseException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class TimeoutException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public TimeoutException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class WebSocketException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public WebSocketException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class ClientClosed extends MarketDataException {
      
      ErrorInfo info;
      public ClientClosed(ErrorInfo info) {
        super(new StringBuilder()
        .append("info=")
        .append(info)
        
        
        .toString());
        this.info = info;
        }

      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class ConfigException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public ConfigException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class ApiException extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public ApiException(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
    
    public static class Other extends MarketDataException {
      
      String msg;
      
      ErrorInfo info;
      public Other(String msg, ErrorInfo info) {
        super(new StringBuilder()
        .append("msg=")
        .append(msg)
        
        .append(", ")
        
        
        .append("info=")
        .append(info)
        
        
        .toString());
        this.msg = msg;
        this.info = info;
        }

      public String msg() {
        return this.msg;
      }
      public ErrorInfo info() {
        return this.info;
      }
      
      
      
    }
     
}

