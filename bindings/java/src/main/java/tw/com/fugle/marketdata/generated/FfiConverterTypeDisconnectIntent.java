package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDisconnectIntent implements FfiConverterRustBuffer<DisconnectIntent> {
    INSTANCE;

    @Override
    public DisconnectIntent read(ByteBuffer buf) {
        try {
            return DisconnectIntent.values()[buf.getInt() - 1];
        } catch (IndexOutOfBoundsException e) {
            throw new RuntimeException("invalid enum value, something is very wrong!!", e);
        }
    }

    @Override
    public long allocationSize(DisconnectIntent value) {
        return 4L;
    }

    @Override
    public void write(DisconnectIntent value, ByteBuffer buf) {
        buf.putInt(value.ordinal() + 1);
    }
}




