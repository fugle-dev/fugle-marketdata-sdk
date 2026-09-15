package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptTotalStats implements FfiConverterRustBuffer<FutOptTotalStats> {
  INSTANCE;

  @Override
  public FutOptTotalStats read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptTotalStats.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptTotalStats value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptTotalStats.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptTotalStats value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptTotalStats.INSTANCE.write(value, buf);
    }
  }
}



