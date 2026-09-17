package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMessageOverflowRecord implements FfiConverterRustBuffer<MessageOverflowRecord> {
    INSTANCE;

    @Override
    public MessageOverflowRecord read(ByteBuffer buf) {
        try {
            return MessageOverflowRecord.values()[buf.getInt() - 1];
        } catch (IndexOutOfBoundsException e) {
            throw new RuntimeException("invalid enum value, something is very wrong!!", e);
        }
    }

    @Override
    public long allocationSize(MessageOverflowRecord value) {
        return 4L;
    }

    @Override
    public void write(MessageOverflowRecord value, ByteBuffer buf) {
        buf.putInt(value.ordinal() + 1);
    }
}




