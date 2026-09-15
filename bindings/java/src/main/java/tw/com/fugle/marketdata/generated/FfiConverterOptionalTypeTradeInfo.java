package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeTradeInfo implements FfiConverterRustBuffer<TradeInfo> {
  INSTANCE;

  @Override
  public TradeInfo read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeTradeInfo.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(TradeInfo value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeTradeInfo.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(TradeInfo value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeTradeInfo.INSTANCE.write(value, buf);
    }
  }
}



