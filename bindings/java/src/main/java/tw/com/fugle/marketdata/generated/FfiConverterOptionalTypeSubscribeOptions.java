package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeSubscribeOptions implements FfiConverterRustBuffer<SubscribeOptions> {
  INSTANCE;

  @Override
  public SubscribeOptions read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeSubscribeOptions.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(SubscribeOptions value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeSubscribeOptions.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(SubscribeOptions value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeSubscribeOptions.INSTANCE.write(value, buf);
    }
  }
}



