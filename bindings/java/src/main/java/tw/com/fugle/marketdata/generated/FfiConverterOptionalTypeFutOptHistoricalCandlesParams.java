package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptHistoricalCandlesParams implements FfiConverterRustBuffer<FutOptHistoricalCandlesParams> {
  INSTANCE;

  @Override
  public FutOptHistoricalCandlesParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptHistoricalCandlesParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptHistoricalCandlesParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptHistoricalCandlesParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptHistoricalCandlesParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptHistoricalCandlesParams.INSTANCE.write(value, buf);
    }
  }
}



