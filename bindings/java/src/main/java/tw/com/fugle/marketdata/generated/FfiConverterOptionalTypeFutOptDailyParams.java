package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptDailyParams implements FfiConverterRustBuffer<FutOptDailyParams> {
  INSTANCE;

  @Override
  public FutOptDailyParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptDailyParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptDailyParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptDailyParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptDailyParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptDailyParams.INSTANCE.write(value, buf);
    }
  }
}



