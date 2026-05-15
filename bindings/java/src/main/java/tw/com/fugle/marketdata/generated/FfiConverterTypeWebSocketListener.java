package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import com.sun.jna.Pointer;

public enum FfiConverterTypeWebSocketListener implements FfiConverter<WebSocketListener, Pointer> {
    INSTANCE;
    public final UniffiHandleMap<WebSocketListener> handleMap = new UniffiHandleMap<>();

    @Override
    public Pointer lower(WebSocketListener value) {
        return new Pointer(handleMap.insert(value));
    }

    @Override
    public WebSocketListener lift(Pointer value) {
        return new WebSocketListenerImpl(value);
    }

    @Override
    public WebSocketListener read(ByteBuffer buf) {
        // The Rust code always writes pointers as 8 bytes, and will
        // fail to compile if they don't fit.
        return lift(new Pointer(buf.getLong()));
    }

    @Override
    public long allocationSize(WebSocketListener value) {
      return 8L;
    }

    @Override
    public void write(WebSocketListener value, ByteBuffer buf) {
        // The Rust code always expects pointers written as 8 bytes,
        // and will fail to compile if they don't fit.
        buf.putLong(Pointer.nativeValue(lower(value)));
    }
}



