package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeProduct implements FfiConverterRustBuffer<Product> {
  INSTANCE;

  @Override
  public Product read(ByteBuffer buf) {
    return new Product(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(Product value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.productType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.underlyingSymbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractType()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.contractSize()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.underlyingType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.statusCode()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.tradingCurrency()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.quoteAcceptable()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.canBlockTrade()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.startDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.expiryType()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.marketCloseGroup()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.endSession())
      );
  }

  @Override
  public void write(Product value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.productType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.underlyingSymbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractType(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.contractSize(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.underlyingType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.statusCode(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.tradingCurrency(), buf);
      FfiConverterBoolean.INSTANCE.write(value.quoteAcceptable(), buf);
      FfiConverterBoolean.INSTANCE.write(value.canBlockTrade(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.startDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.expiryType(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.marketCloseGroup(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.endSession(), buf);
  }
}



