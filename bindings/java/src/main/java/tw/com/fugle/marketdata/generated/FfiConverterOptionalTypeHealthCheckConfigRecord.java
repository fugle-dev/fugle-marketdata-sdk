package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeHealthCheckConfigRecord implements FfiConverterRustBuffer<HealthCheckConfigRecord> {
  INSTANCE;

  @Override
  public HealthCheckConfigRecord read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeHealthCheckConfigRecord.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(HealthCheckConfigRecord value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeHealthCheckConfigRecord.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(HealthCheckConfigRecord value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeHealthCheckConfigRecord.INSTANCE.write(value, buf);
    }
  }
}



