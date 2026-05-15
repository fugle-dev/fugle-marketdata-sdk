package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeReconnectConfigRecord implements FfiConverterRustBuffer<ReconnectConfigRecord> {
  INSTANCE;

  @Override
  public ReconnectConfigRecord read(ByteBuffer buf) {
    return new ReconnectConfigRecord(
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ReconnectConfigRecord value) {
      return (
            FfiConverterInteger.INSTANCE.allocationSize(value.maxAttempts()) +
            FfiConverterLong.INSTANCE.allocationSize(value.initialDelayMs()) +
            FfiConverterLong.INSTANCE.allocationSize(value.maxDelayMs())
      );
  }

  @Override
  public void write(ReconnectConfigRecord value, ByteBuffer buf) {
      FfiConverterInteger.INSTANCE.write(value.maxAttempts(), buf);
      FfiConverterLong.INSTANCE.write(value.initialDelayMs(), buf);
      FfiConverterLong.INSTANCE.write(value.maxDelayMs(), buf);
  }
}



