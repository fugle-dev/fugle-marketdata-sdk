package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptTickersParams implements FfiConverterRustBuffer<FutOptTickersParams> {
  INSTANCE;

  @Override
  public FutOptTickersParams read(ByteBuffer buf) {
    return new FutOptTickersParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptTickersParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.product()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractType()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isSpread())
      );
  }

  @Override
  public void write(FutOptTickersParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.product(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractType(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isSpread(), buf);
  }
}



