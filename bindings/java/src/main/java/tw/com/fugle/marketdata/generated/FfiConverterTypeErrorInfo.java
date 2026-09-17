package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeErrorInfo implements FfiConverterRustBuffer<ErrorInfo> {
  INSTANCE;

  @Override
  public ErrorInfo read(ByteBuffer buf) {
    return new ErrorInfo(
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterTypeErrorSourceKind.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalShort.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterMapStringString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ErrorInfo value) {
      return (
            FfiConverterInteger.INSTANCE.allocationSize(value.code()) +
            FfiConverterTypeErrorSourceKind.INSTANCE.allocationSize(value.sourceKind()) +
            FfiConverterString.INSTANCE.allocationSize(value.message()) +
            FfiConverterOptionalShort.INSTANCE.allocationSize(value.status()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.body()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.requestId()) +
            FfiConverterMapStringString.INSTANCE.allocationSize(value.headers())
      );
  }

  @Override
  public void write(ErrorInfo value, ByteBuffer buf) {
      FfiConverterInteger.INSTANCE.write(value.code(), buf);
      FfiConverterTypeErrorSourceKind.INSTANCE.write(value.sourceKind(), buf);
      FfiConverterString.INSTANCE.write(value.message(), buf);
      FfiConverterOptionalShort.INSTANCE.write(value.status(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.body(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.requestId(), buf);
      FfiConverterMapStringString.INSTANCE.write(value.headers(), buf);
  }
}



