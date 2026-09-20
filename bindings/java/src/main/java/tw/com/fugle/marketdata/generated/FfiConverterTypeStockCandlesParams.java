package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStockCandlesParams implements FfiConverterRustBuffer<StockCandlesParams> {
  INSTANCE;

  @Override
  public StockCandlesParams read(ByteBuffer buf) {
    return new StockCandlesParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StockCandlesParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.oddLot()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sort())
      );
  }

  @Override
  public void write(StockCandlesParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.oddLot(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sort(), buf);
  }
}



