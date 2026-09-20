package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeMoversParams implements FfiConverterRustBuffer<MoversParams> {
  INSTANCE;

  @Override
  public MoversParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeMoversParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(MoversParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeMoversParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(MoversParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeMoversParams.INSTANCE.write(value, buf);
    }
  }
}



