package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeStockHistoricalCandlesParams implements FfiConverterRustBuffer<StockHistoricalCandlesParams> {
  INSTANCE;

  @Override
  public StockHistoricalCandlesParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeStockHistoricalCandlesParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(StockHistoricalCandlesParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeStockHistoricalCandlesParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(StockHistoricalCandlesParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeStockHistoricalCandlesParams.INSTANCE.write(value, buf);
    }
  }
}



