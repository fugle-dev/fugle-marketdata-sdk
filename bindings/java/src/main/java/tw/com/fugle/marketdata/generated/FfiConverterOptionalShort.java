package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalShort implements FfiConverterRustBuffer<Short> {
  INSTANCE;

  @Override
  public Short read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterShort.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(Short value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterShort.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(Short value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterShort.INSTANCE.write(value, buf);
    }
  }
}



