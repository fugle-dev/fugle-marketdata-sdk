package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTlsConfigRecord implements FfiConverterRustBuffer<TlsConfigRecord> {
  INSTANCE;

  @Override
  public TlsConfigRecord read(ByteBuffer buf) {
    return new TlsConfigRecord(
      FfiConverterOptionalByteArray.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TlsConfigRecord value) {
      return (
            FfiConverterOptionalByteArray.INSTANCE.allocationSize(value.rootCertPem()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.acceptInvalidCerts())
      );
  }

  @Override
  public void write(TlsConfigRecord value, ByteBuffer buf) {
      FfiConverterOptionalByteArray.INSTANCE.write(value.rootCertPem(), buf);
      FfiConverterBoolean.INSTANCE.write(value.acceptInvalidCerts(), buf);
  }
}



