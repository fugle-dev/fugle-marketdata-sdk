package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalDouble implements FfiConverterRustBuffer<Double> {
  INSTANCE;

  @Override
  public Double read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterDouble.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(Double value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterDouble.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(Double value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterDouble.INSTANCE.write(value, buf);
    }
  }
}



