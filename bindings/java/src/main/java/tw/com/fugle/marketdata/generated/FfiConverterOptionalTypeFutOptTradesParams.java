package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptTradesParams implements FfiConverterRustBuffer<FutOptTradesParams> {
  INSTANCE;

  @Override
  public FutOptTradesParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptTradesParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptTradesParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptTradesParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptTradesParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptTradesParams.INSTANCE.write(value, buf);
    }
  }
}



