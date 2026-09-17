package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeErrorSourceKind implements FfiConverterRustBuffer<ErrorSourceKind> {
    INSTANCE;

    @Override
    public ErrorSourceKind read(ByteBuffer buf) {
        try {
            return ErrorSourceKind.values()[buf.getInt() - 1];
        } catch (IndexOutOfBoundsException e) {
            throw new RuntimeException("invalid enum value, something is very wrong!!", e);
        }
    }

    @Override
    public long allocationSize(ErrorSourceKind value) {
        return 4L;
    }

    @Override
    public void write(ErrorSourceKind value, ByteBuffer buf) {
        buf.putInt(value.ordinal() + 1);
    }
}




