package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeWebSocketEndpoint implements FfiConverterRustBuffer<WebSocketEndpoint> {
    INSTANCE;

    @Override
    public WebSocketEndpoint read(ByteBuffer buf) {
        try {
            return WebSocketEndpoint.values()[buf.getInt() - 1];
        } catch (IndexOutOfBoundsException e) {
            throw new RuntimeException("invalid enum value, something is very wrong!!", e);
        }
    }

    @Override
    public long allocationSize(WebSocketEndpoint value) {
        return 4L;
    }

    @Override
    public void write(WebSocketEndpoint value, ByteBuffer buf) {
        buf.putInt(value.ordinal() + 1);
    }
}





