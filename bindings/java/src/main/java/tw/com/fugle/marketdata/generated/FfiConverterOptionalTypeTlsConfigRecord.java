package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeTlsConfigRecord implements FfiConverterRustBuffer<TlsConfigRecord> {
  INSTANCE;

  @Override
  public TlsConfigRecord read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeTlsConfigRecord.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(TlsConfigRecord value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeTlsConfigRecord.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(TlsConfigRecord value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeTlsConfigRecord.INSTANCE.write(value, buf);
    }
  }
}



