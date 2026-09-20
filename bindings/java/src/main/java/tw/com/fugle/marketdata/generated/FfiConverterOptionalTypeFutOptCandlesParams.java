package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptCandlesParams implements FfiConverterRustBuffer<FutOptCandlesParams> {
  INSTANCE;

  @Override
  public FutOptCandlesParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptCandlesParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptCandlesParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptCandlesParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptCandlesParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptCandlesParams.INSTANCE.write(value, buf);
    }
  }
}



