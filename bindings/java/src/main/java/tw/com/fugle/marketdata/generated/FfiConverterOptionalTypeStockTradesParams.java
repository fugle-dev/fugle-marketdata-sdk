package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeStockTradesParams implements FfiConverterRustBuffer<StockTradesParams> {
  INSTANCE;

  @Override
  public StockTradesParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeStockTradesParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(StockTradesParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeStockTradesParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(StockTradesParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeStockTradesParams.INSTANCE.write(value, buf);
    }
  }
}



