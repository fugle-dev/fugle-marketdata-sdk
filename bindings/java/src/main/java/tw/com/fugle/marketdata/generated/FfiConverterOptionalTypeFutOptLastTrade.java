package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptLastTrade implements FfiConverterRustBuffer<FutOptLastTrade> {
  INSTANCE;

  @Override
  public FutOptLastTrade read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptLastTrade.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptLastTrade value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptLastTrade.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptLastTrade value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptLastTrade.INSTANCE.write(value, buf);
    }
  }
}



