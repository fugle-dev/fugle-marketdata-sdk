package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import com.sun.jna.*;
import com.sun.jna.ptr.*;
/**
 * FutOpt market data client
 */
public interface FutOptClientInterface {
    
    /**
     * Access historical data endpoints
     */
    public FutOptHistoricalClient historical();
    
    /**
     * Access intraday (real-time) endpoints
     */
    public FutOptIntradayClient intraday();
    
}

