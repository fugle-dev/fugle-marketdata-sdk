package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMessageQueueConfigRecord implements FfiConverterRustBuffer<MessageQueueConfigRecord> {
  INSTANCE;

  @Override
  public MessageQueueConfigRecord read(ByteBuffer buf) {
    return new MessageQueueConfigRecord(
      FfiConverterTypeMessageOverflowRecord.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(MessageQueueConfigRecord value) {
      return (
            FfiConverterTypeMessageOverflowRecord.INSTANCE.allocationSize(value.overflow()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.buffer())
      );
  }

  @Override
  public void write(MessageQueueConfigRecord value, ByteBuffer buf) {
      FfiConverterTypeMessageOverflowRecord.INSTANCE.write(value.overflow(), buf);
      FfiConverterInteger.INSTANCE.write(value.buffer(), buf);
  }
}



