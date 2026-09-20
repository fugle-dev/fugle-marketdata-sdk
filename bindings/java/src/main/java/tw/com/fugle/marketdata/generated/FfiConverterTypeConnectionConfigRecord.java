package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeConnectionConfigRecord implements FfiConverterRustBuffer<ConnectionConfigRecord> {
  INSTANCE;

  @Override
  public ConnectionConfigRecord read(ByteBuffer buf) {
    return new ConnectionConfigRecord(
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ConnectionConfigRecord value) {
      return (
            FfiConverterLong.INSTANCE.allocationSize(value.authTimeoutMs())
      );
  }

  @Override
  public void write(ConnectionConfigRecord value, ByteBuffer buf) {
      FfiConverterLong.INSTANCE.write(value.authTimeoutMs(), buf);
  }
}



