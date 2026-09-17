package tw.com.fugle.marketdata.generated;


import com.sun.jna.Structure;
import com.sun.jna.Pointer;

@Structure.FieldOrder({ "onConnected", "onAuthenticated", "onUnauthenticated", "onDisconnected", "onMessage", "onError", "onReconnecting", "onReconnectFailed", "onMessagesDropped", "uniffiFree" })
public class UniffiVTableCallbackInterfaceWebSocketListener extends Structure {
    public UniffiCallbackInterfaceWebSocketListenerMethod0 onConnected = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod1 onAuthenticated = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod2 onUnauthenticated = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod3 onDisconnected = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod4 onMessage = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod5 onError = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod6 onReconnecting = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod7 onReconnectFailed = null;
    public UniffiCallbackInterfaceWebSocketListenerMethod8 onMessagesDropped = null;
    public UniffiCallbackInterfaceFree uniffiFree = null;

    // no-arg constructor required so JNA can instantiate and reflect
    public UniffiVTableCallbackInterfaceWebSocketListener() {
        super();
    }
    
    public UniffiVTableCallbackInterfaceWebSocketListener(
        UniffiCallbackInterfaceWebSocketListenerMethod0 onConnected,
        UniffiCallbackInterfaceWebSocketListenerMethod1 onAuthenticated,
        UniffiCallbackInterfaceWebSocketListenerMethod2 onUnauthenticated,
        UniffiCallbackInterfaceWebSocketListenerMethod3 onDisconnected,
        UniffiCallbackInterfaceWebSocketListenerMethod4 onMessage,
        UniffiCallbackInterfaceWebSocketListenerMethod5 onError,
        UniffiCallbackInterfaceWebSocketListenerMethod6 onReconnecting,
        UniffiCallbackInterfaceWebSocketListenerMethod7 onReconnectFailed,
        UniffiCallbackInterfaceWebSocketListenerMethod8 onMessagesDropped,
        UniffiCallbackInterfaceFree uniffiFree
    ) {
        this.onConnected = onConnected;
        this.onAuthenticated = onAuthenticated;
        this.onUnauthenticated = onUnauthenticated;
        this.onDisconnected = onDisconnected;
        this.onMessage = onMessage;
        this.onError = onError;
        this.onReconnecting = onReconnecting;
        this.onReconnectFailed = onReconnectFailed;
        this.onMessagesDropped = onMessagesDropped;
        this.uniffiFree = uniffiFree;
    }

    public static class UniffiByValue extends UniffiVTableCallbackInterfaceWebSocketListener implements Structure.ByValue {
        public UniffiByValue(
            UniffiCallbackInterfaceWebSocketListenerMethod0 onConnected,
            UniffiCallbackInterfaceWebSocketListenerMethod1 onAuthenticated,
            UniffiCallbackInterfaceWebSocketListenerMethod2 onUnauthenticated,
            UniffiCallbackInterfaceWebSocketListenerMethod3 onDisconnected,
            UniffiCallbackInterfaceWebSocketListenerMethod4 onMessage,
            UniffiCallbackInterfaceWebSocketListenerMethod5 onError,
            UniffiCallbackInterfaceWebSocketListenerMethod6 onReconnecting,
            UniffiCallbackInterfaceWebSocketListenerMethod7 onReconnectFailed,
            UniffiCallbackInterfaceWebSocketListenerMethod8 onMessagesDropped,
            UniffiCallbackInterfaceFree uniffiFree
        ) {
            super(onConnected,        
            onAuthenticated,        
            onUnauthenticated,        
            onDisconnected,        
            onMessage,        
            onError,        
            onReconnecting,        
            onReconnectFailed,        
            onMessagesDropped,        
            uniffiFree        
            );
        }
    }

    void uniffiSetValue(UniffiVTableCallbackInterfaceWebSocketListener other) {
        onConnected = other.onConnected;
        onAuthenticated = other.onAuthenticated;
        onUnauthenticated = other.onUnauthenticated;
        onDisconnected = other.onDisconnected;
        onMessage = other.onMessage;
        onError = other.onError;
        onReconnecting = other.onReconnecting;
        onReconnectFailed = other.onReconnectFailed;
        onMessagesDropped = other.onMessagesDropped;
        uniffiFree = other.uniffiFree;
    }

}















































































































































































































































































































