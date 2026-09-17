package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeCredentialKind implements FfiConverterRustBuffer<CredentialKind> {
    INSTANCE;

    @Override
    public CredentialKind read(ByteBuffer buf) {
        try {
            return CredentialKind.values()[buf.getInt() - 1];
        } catch (IndexOutOfBoundsException e) {
            throw new RuntimeException("invalid enum value, something is very wrong!!", e);
        }
    }

    @Override
    public long allocationSize(CredentialKind value) {
        return 4L;
    }

    @Override
    public void write(CredentialKind value, ByteBuffer buf) {
        buf.putInt(value.ordinal() + 1);
    }
}




