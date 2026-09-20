package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStockTradesParams implements FfiConverterRustBuffer<StockTradesParams> {
  INSTANCE;

  @Override
  public StockTradesParams read(ByteBuffer buf) {
    return new StockTradesParams(
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StockTradesParams value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.oddLot()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.offset()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.limit()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sort()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isTrial())
      );
  }

  @Override
  public void write(StockTradesParams value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.oddLot(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.offset(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.limit(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sort(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isTrial(), buf);
  }
}



