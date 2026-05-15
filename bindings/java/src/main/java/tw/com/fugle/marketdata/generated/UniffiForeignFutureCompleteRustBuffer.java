package tw.com.fugle.marketdata.generated;


import com.sun.jna.*;
import com.sun.jna.ptr.*;

interface UniffiForeignFutureCompleteRustBuffer extends Callback {
    public void callback(long callbackData,UniffiForeignFutureStructRustBuffer.UniffiByValue result);
}
