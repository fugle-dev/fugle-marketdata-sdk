package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStockTickersParams implements FfiConverterRustBuffer<StockTickersParams> {
  INSTANCE;

  @Override
  public StockTickersParams read(ByteBuffer buf) {
    return new StockTickersParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StockTickersParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.industry()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isNormal()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isAttention()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isDisposition()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isHalted()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.symbol())
      );
  }

  @Override
  public void write(StockTickersParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.industry(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isNormal(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isAttention(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isDisposition(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isHalted(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.symbol(), buf);
  }
}



