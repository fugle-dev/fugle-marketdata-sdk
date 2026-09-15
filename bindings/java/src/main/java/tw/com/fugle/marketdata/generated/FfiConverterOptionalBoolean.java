package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalBoolean implements FfiConverterRustBuffer<Boolean> {
  INSTANCE;

  @Override
  public Boolean read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterBoolean.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(Boolean value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterBoolean.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(Boolean value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterBoolean.INSTANCE.write(value, buf);
    }
  }
}



