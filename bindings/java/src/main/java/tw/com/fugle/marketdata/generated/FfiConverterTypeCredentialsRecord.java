package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeCredentialsRecord implements FfiConverterRustBuffer<CredentialsRecord> {
  INSTANCE;

  @Override
  public CredentialsRecord read(ByteBuffer buf) {
    return new CredentialsRecord(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(CredentialsRecord value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.apiKey()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.bearerToken()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sdkToken())
      );
  }

  @Override
  public void write(CredentialsRecord value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.apiKey(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.bearerToken(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sdkToken(), buf);
  }
}



