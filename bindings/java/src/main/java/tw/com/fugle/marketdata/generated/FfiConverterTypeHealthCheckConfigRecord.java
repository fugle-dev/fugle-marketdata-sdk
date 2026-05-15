package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeHealthCheckConfigRecord implements FfiConverterRustBuffer<HealthCheckConfigRecord> {
  INSTANCE;

  @Override
  public HealthCheckConfigRecord read(ByteBuffer buf) {
    return new HealthCheckConfigRecord(
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(HealthCheckConfigRecord value) {
      return (
            FfiConverterBoolean.INSTANCE.allocationSize(value.enabled()) +
            FfiConverterLong.INSTANCE.allocationSize(value.intervalMs()) +
            FfiConverterLong.INSTANCE.allocationSize(value.maxMissedPongs())
      );
  }

  @Override
  public void write(HealthCheckConfigRecord value, ByteBuffer buf) {
      FfiConverterBoolean.INSTANCE.write(value.enabled(), buf);
      FfiConverterLong.INSTANCE.write(value.intervalMs(), buf);
      FfiConverterLong.INSTANCE.write(value.maxMissedPongs(), buf);
  }
}



