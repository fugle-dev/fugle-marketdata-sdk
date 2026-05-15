package tw.com.fugle.marketdata.generated;


import com.sun.jna.Structure;
import com.sun.jna.Pointer;

@Structure.FieldOrder({ "onConnected", "onDisconnected", "onMessage", "onError", "onReconnecting", "onReconnectFailed", "uniffiFree" })
public class UniffiVTableCallbackInterfaceWebSocketListener extends Structure {
    public UniffiCallbackInterfaceWebSocketListenerMethod0 onConnected = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod1 onDisconnected = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod2 onMessage = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod3 onError = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod4 onReconnecting = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod5 onReconnectFailed = null;
    public UniffiCallbackInterfaceFree uniffiFree = null;

    // no-arg constructor required so JNA can instantiate and reflect
    public UniffiVTableCallbackInterfaceWebSocketListener() {
        super();
    }
    
    public UniffiVTableCallbackInterfaceWebSocketListener(
        UniffiCallbackInterfaceWebSocketListenerMethod0 onConnected,
        UniffiCallbackInterfaceWebSocketListenerMethod1 onDisconnected,
        UniffiCallbackInterfaceWebSocketListenerMethod2 onMessage,
        UniffiCallbackInterfaceWebSocketListenerMethod3 onError,
        UniffiCallbackInterfaceWebSocketListenerMethod4 onReconnecting,
        UniffiCallbackInterfaceWebSocketListenerMethod5 onReconnectFailed,
        UniffiCallbackInterfaceFree uniffiFree
    ) {
        this.onConnected = onConnected;
        this.onDisconnected = onDisconnected;
        this.onMessage = onMessage;
        this.onError = onError;
        this.onReconnecting = onReconnecting;
        this.onReconnectFailed = onReconnectFailed;
        this.uniffiFree = uniffiFree;
    }

    public static class UniffiByValue extends UniffiVTableCallbackInterfaceWebSocketListener implements Structure.ByValue {
        public UniffiByValue(
            UniffiCallbackInterfaceWebSocketListenerMethod0 onConnected,
            UniffiCallbackInterfaceWebSocketListenerMethod1 onDisconnected,
            UniffiCallbackInterfaceWebSocketListenerMethod2 onMessage,
            UniffiCallbackInterfaceWebSocketListenerMethod3 onError,
            UniffiCallbackInterfaceWebSocketListenerMethod4 onReconnecting,
            UniffiCallbackInterfaceWebSocketListenerMethod5 onReconnectFailed,
            UniffiCallbackInterfaceFree uniffiFree
        ) {
            super(onConnected,        
            onDisconnected,        
            onMessage,        
            onError,        
            onReconnecting,        
            onReconnectFailed,        
            uniffiFree        
            );
        }
    }

    void uniffiSetValue(UniffiVTableCallbackInterfaceWebSocketListener other) {
        onConnected = other.onConnected;
        onDisconnected = other.onDisconnected;
        onMessage = other.onMessage;
        onError = other.onError;
        onReconnecting = other.onReconnecting;
        onReconnectFailed = other.onReconnectFailed;
        uniffiFree = other.uniffiFree;
    }

}













































































































































































































































































