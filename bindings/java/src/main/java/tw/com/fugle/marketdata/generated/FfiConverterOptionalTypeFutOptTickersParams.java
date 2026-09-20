package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptTickersParams implements FfiConverterRustBuffer<FutOptTickersParams> {
  INSTANCE;

  @Override
  public FutOptTickersParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptTickersParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptTickersParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptTickersParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptTickersParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptTickersParams.INSTANCE.write(value, buf);
    }
  }
}



