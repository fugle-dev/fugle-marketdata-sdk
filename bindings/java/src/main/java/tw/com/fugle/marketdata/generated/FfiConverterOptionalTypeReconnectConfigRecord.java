package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeReconnectConfigRecord implements FfiConverterRustBuffer<ReconnectConfigRecord> {
  INSTANCE;

  @Override
  public ReconnectConfigRecord read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeReconnectConfigRecord.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(ReconnectConfigRecord value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeReconnectConfigRecord.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(ReconnectConfigRecord value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeReconnectConfigRecord.INSTANCE.write(value, buf);
    }
  }
}



