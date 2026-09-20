package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeTechnicalParams implements FfiConverterRustBuffer<TechnicalParams> {
  INSTANCE;

  @Override
  public TechnicalParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeTechnicalParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(TechnicalParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeTechnicalParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(TechnicalParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeTechnicalParams.INSTANCE.write(value, buf);
    }
  }
}



