package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeTotalStats implements FfiConverterRustBuffer<TotalStats> {
  INSTANCE;

  @Override
  public TotalStats read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeTotalStats.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(TotalStats value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeTotalStats.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(TotalStats value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeTotalStats.INSTANCE.write(value, buf);
    }
  }
}



