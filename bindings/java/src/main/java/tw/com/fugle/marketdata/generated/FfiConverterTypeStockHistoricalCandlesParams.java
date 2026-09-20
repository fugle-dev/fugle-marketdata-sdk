package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStockHistoricalCandlesParams implements FfiConverterRustBuffer<StockHistoricalCandlesParams> {
  INSTANCE;

  @Override
  public StockHistoricalCandlesParams read(ByteBuffer buf) {
    return new StockHistoricalCandlesParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StockHistoricalCandlesParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.from()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.to()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.fields()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sort()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.adjusted())
      );
  }

  @Override
  public void write(StockHistoricalCandlesParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.from(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.to(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.fields(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sort(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.adjusted(), buf);
  }
}



