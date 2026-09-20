package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeOwnershipParams implements FfiConverterRustBuffer<OwnershipParams> {
  INSTANCE;

  @Override
  public OwnershipParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeOwnershipParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(OwnershipParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeOwnershipParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(OwnershipParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeOwnershipParams.INSTANCE.write(value, buf);
    }
  }
}



