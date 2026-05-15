package tw.com.fugle.marketdata.generated;


import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Callable;
import java.util.function.Function;
import java.util.function.Consumer;
import java.util.function.Supplier;
import java.util.List;
import com.sun.jna.*;
import com.sun.jna.ptr.*;

// Put the implementation in an object so we don't pollute the top-level namespace
public class UniffiCallbackInterfaceWebSocketListener {
    public static final UniffiCallbackInterfaceWebSocketListener INSTANCE = new UniffiCallbackInterfaceWebSocketListener();
    UniffiVTableCallbackInterfaceWebSocketListener.UniffiByValue vtable;
    
    UniffiCallbackInterfaceWebSocketListener() {
        vtable = new UniffiVTableCallbackInterfaceWebSocketListener.UniffiByValue(
            onConnected.INSTANCE,
            onDisconnected.INSTANCE,
            onMessage.INSTANCE,
            onError.INSTANCE,
            onReconnecting.INSTANCE,
            onReconnectFailed.INSTANCE,
            UniffiFree.INSTANCE
        );
    }
    
    // Registers the foreign callback with the Rust side.
    // This method is generated for each callback interface.
    void register(UniffiLib lib) {
        lib.uniffi_marketdata_uniffi_fn_init_callback_vtable_websocketlistener(vtable);
    }
    
    public static class onConnected implements UniffiCallbackInterfaceWebSocketListenerMethod0 {
        public static final onConnected INSTANCE = new onConnected();
        private onConnected() {}

        @Override
        public void callback(long uniffiHandle,Pointer uniffiOutReturn,UniffiRustCallStatus uniffiCallStatus) {
            var uniffiObj = FfiConverterTypeWebSocketListener.INSTANCE.handleMap.get(uniffiHandle);
            Supplier<Void> makeCall = () -> {
                uniffiObj.onConnected(
                );
                return null;
            };
            Consumer<Void> writeReturn = (nothing) -> {};
            UniffiHelpers.uniffiTraitInterfaceCall(uniffiCallStatus, makeCall, writeReturn);
        }
    }
    
    public static class onDisconnected implements UniffiCallbackInterfaceWebSocketListenerMethod1 {
        public static final onDisconnected INSTANCE = new onDisconnected();
        private onDisconnected() {}

        @Override
        public void callback(long uniffiHandle,Pointer uniffiOutReturn,UniffiRustCallStatus uniffiCallStatus) {
            var uniffiObj = FfiConverterTypeWebSocketListener.INSTANCE.handleMap.get(uniffiHandle);
            Supplier<Void> makeCall = () -> {
                uniffiObj.onDisconnected(
                );
                return null;
            };
            Consumer<Void> writeReturn = (nothing) -> {};
            UniffiHelpers.uniffiTraitInterfaceCall(uniffiCallStatus, makeCall, writeReturn);
        }
    }
    
    public static class onMessage implements UniffiCallbackInterfaceWebSocketListenerMethod2 {
        public static final onMessage INSTANCE = new onMessage();
        private onMessage() {}

        @Override
        public void callback(long uniffiHandle,RustBuffer.ByValue message,Pointer uniffiOutReturn,UniffiRustCallStatus uniffiCallStatus) {
            var uniffiObj = FfiConverterTypeWebSocketListener.INSTANCE.handleMap.get(uniffiHandle);
            Supplier<Void> makeCall = () -> {
                uniffiObj.onMessage(
                    FfiConverterTypeStreamMessage.INSTANCE.lift(message)
                );
                return null;
            };
            Consumer<Void> writeReturn = (nothing) -> {};
            UniffiHelpers.uniffiTraitInterfaceCall(uniffiCallStatus, makeCall, writeReturn);
        }
    }
    
    public static class onError implements UniffiCallbackInterfaceWebSocketListenerMethod3 {
        public static final onError INSTANCE = new onError();
        private onError() {}

        @Override
        public void callback(long uniffiHandle,RustBuffer.ByValue errorMessage,Pointer uniffiOutReturn,UniffiRustCallStatus uniffiCallStatus) {
            var uniffiObj = FfiConverterTypeWebSocketListener.INSTANCE.handleMap.get(uniffiHandle);
            Supplier<Void> makeCall = () -> {
                uniffiObj.onError(
                    FfiConverterString.INSTANCE.lift(errorMessage)
                );
                return null;
            };
            Consumer<Void> writeReturn = (nothing) -> {};
            UniffiHelpers.uniffiTraitInterfaceCall(uniffiCallStatus, makeCall, writeReturn);
        }
    }
    
    public static class onReconnecting implements UniffiCallbackInterfaceWebSocketListenerMethod4 {
        public static final onReconnecting INSTANCE = new onReconnecting();
        private onReconnecting() {}

        @Override
        public void callback(long uniffiHandle,int attempt,Pointer uniffiOutReturn,UniffiRustCallStatus uniffiCallStatus) {
            var uniffiObj = FfiConverterTypeWebSocketListener.INSTANCE.handleMap.get(uniffiHandle);
            Supplier<Void> makeCall = () -> {
                uniffiObj.onReconnecting(
                    FfiConverterInteger.INSTANCE.lift(attempt)
                );
                return null;
            };
            Consumer<Void> writeReturn = (nothing) -> {};
            UniffiHelpers.uniffiTraitInterfaceCall(uniffiCallStatus, makeCall, writeReturn);
        }
    }
    
    public static class onReconnectFailed implements UniffiCallbackInterfaceWebSocketListenerMethod5 {
        public static final onReconnectFailed INSTANCE = new onReconnectFailed();
        private onReconnectFailed() {}

        @Override
        public void callback(long uniffiHandle,int attempts,Pointer uniffiOutReturn,UniffiRustCallStatus uniffiCallStatus) {
            var uniffiObj = FfiConverterTypeWebSocketListener.INSTANCE.handleMap.get(uniffiHandle);
            Supplier<Void> makeCall = () -> {
                uniffiObj.onReconnectFailed(
                    FfiConverterInteger.INSTANCE.lift(attempts)
                );
                return null;
            };
            Consumer<Void> writeReturn = (nothing) -> {};
            UniffiHelpers.uniffiTraitInterfaceCall(uniffiCallStatus, makeCall, writeReturn);
        }
    }

    public static class UniffiFree implements UniffiCallbackInterfaceFree {
        public static final UniffiFree INSTANCE = new UniffiFree();

        private UniffiFree() {}

        @Override
        public void callback(long handle) {
            FfiConverterTypeWebSocketListener.INSTANCE.handleMap.remove(handle);
        }
    }
}

