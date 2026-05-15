package tw.com.fugle.marketdata.generated;


import com.sun.jna.*;
import com.sun.jna.ptr.*;

interface UniffiCallbackInterfaceWebSocketListenerMethod2 extends Callback {
    public void callback(long uniffiHandle,RustBuffer.ByValue message,Pointer uniffiOutReturn,
        UniffiRustCallStatus uniffiCallStatus);
}
