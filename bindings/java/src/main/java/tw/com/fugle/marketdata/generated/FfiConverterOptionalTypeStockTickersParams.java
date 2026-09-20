package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeStockTickersParams implements FfiConverterRustBuffer<StockTickersParams> {
  INSTANCE;

  @Override
  public StockTickersParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeStockTickersParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(StockTickersParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeStockTickersParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(StockTickersParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeStockTickersParams.INSTANCE.write(value, buf);
    }
  }
}



