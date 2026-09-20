package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeStockCandlesParams implements FfiConverterRustBuffer<StockCandlesParams> {
  INSTANCE;

  @Override
  public StockCandlesParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeStockCandlesParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(StockCandlesParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeStockCandlesParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(StockCandlesParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeStockCandlesParams.INSTANCE.write(value, buf);
    }
  }
}



