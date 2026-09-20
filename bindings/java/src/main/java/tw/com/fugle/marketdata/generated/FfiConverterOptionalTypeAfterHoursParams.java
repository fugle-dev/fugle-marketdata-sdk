package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeAfterHoursParams implements FfiConverterRustBuffer<AfterHoursParams> {
  INSTANCE;

  @Override
  public AfterHoursParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeAfterHoursParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(AfterHoursParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeAfterHoursParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(AfterHoursParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeAfterHoursParams.INSTANCE.write(value, buf);
    }
  }
}



