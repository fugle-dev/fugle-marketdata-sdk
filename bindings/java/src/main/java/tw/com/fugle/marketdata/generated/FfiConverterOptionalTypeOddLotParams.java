package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeOddLotParams implements FfiConverterRustBuffer<OddLotParams> {
  INSTANCE;

  @Override
  public OddLotParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeOddLotParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(OddLotParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeOddLotParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(OddLotParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeOddLotParams.INSTANCE.write(value, buf);
    }
  }
}



