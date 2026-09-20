package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptProductsParams implements FfiConverterRustBuffer<FutOptProductsParams> {
  INSTANCE;

  @Override
  public FutOptProductsParams read(ByteBuffer buf) {
    return new FutOptProductsParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptProductsParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.status())
      );
  }

  @Override
  public void write(FutOptProductsParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.status(), buf);
  }
}



