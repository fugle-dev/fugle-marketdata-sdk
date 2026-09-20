package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptHistoricalCandlesParams implements FfiConverterRustBuffer<FutOptHistoricalCandlesParams> {
  INSTANCE;

  @Override
  public FutOptHistoricalCandlesParams read(ByteBuffer buf) {
    return new FutOptHistoricalCandlesParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptHistoricalCandlesParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.from()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.to()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractMonth()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.fields()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sort()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.strikePrice()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.callPut()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours())
      );
  }

  @Override
  public void write(FutOptHistoricalCandlesParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.from(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.to(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractMonth(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.fields(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sort(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.strikePrice(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.callPut(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
  }
}



