package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTechnicalParams implements FfiConverterRustBuffer<TechnicalParams> {
  INSTANCE;

  @Override
  public TechnicalParams read(ByteBuffer buf) {
    return new TechnicalParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TechnicalParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.from()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.to()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe())
      );
  }

  @Override
  public void write(TechnicalParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.from(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.to(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
  }
}



