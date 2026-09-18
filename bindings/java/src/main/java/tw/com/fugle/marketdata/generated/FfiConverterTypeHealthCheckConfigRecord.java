package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeHealthCheckConfigRecord implements FfiConverterRustBuffer<HealthCheckConfigRecord> {
  INSTANCE;

  @Override
  public HealthCheckConfigRecord read(ByteBuffer buf) {
    return new HealthCheckConfigRecord(
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(HealthCheckConfigRecord value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.enabled()) +
            FfiConverterLong.INSTANCE.allocationSize(value.heartbeatTimeoutMs()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.probeEnabled()) +
            FfiConverterLong.INSTANCE.allocationSize(value.idleProbeAfterMs()) +
            FfiConverterLong.INSTANCE.allocationSize(value.probeTimeoutMs())
      );
  }

  @Override
  public void write(HealthCheckConfigRecord value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.enabled(), buf);
      FfiConverterLong.INSTANCE.write(value.heartbeatTimeoutMs(), buf);
      FfiConverterBoolean.INSTANCE.write(value.probeEnabled(), buf);
      FfiConverterLong.INSTANCE.write(value.idleProbeAfterMs(), buf);
      FfiConverterLong.INSTANCE.write(value.probeTimeoutMs(), buf);
  }
}



