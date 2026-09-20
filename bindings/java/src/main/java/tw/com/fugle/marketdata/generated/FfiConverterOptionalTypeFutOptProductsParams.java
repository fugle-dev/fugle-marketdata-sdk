package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeFutOptProductsParams implements FfiConverterRustBuffer<FutOptProductsParams> {
  INSTANCE;

  @Override
  public FutOptProductsParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeFutOptProductsParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(FutOptProductsParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeFutOptProductsParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(FutOptProductsParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeFutOptProductsParams.INSTANCE.write(value, buf);
    }
  }
}



