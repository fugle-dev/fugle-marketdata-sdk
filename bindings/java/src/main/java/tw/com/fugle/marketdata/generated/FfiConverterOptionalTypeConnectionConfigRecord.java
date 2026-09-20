package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeConnectionConfigRecord implements FfiConverterRustBuffer<ConnectionConfigRecord> {
  INSTANCE;

  @Override
  public ConnectionConfigRecord read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeConnectionConfigRecord.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(ConnectionConfigRecord value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeConnectionConfigRecord.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(ConnectionConfigRecord value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeConnectionConfigRecord.INSTANCE.write(value, buf);
    }
  }
}



