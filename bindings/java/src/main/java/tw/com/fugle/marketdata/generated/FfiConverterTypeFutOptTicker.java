package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptTicker implements FfiConverterRustBuffer<FutOptTicker> {
  INSTANCE;

  @Override
  public FutOptTicker read(ByteBuffer buf) {
    return new FutOptTicker(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptTicker value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.referencePrice()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.startDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.endDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.settlementDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractSubType()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isDynamicBanding()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.flowGroup())
      );
  }

  @Override
  public void write(FutOptTicker value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.referencePrice(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.startDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.endDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.settlementDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractSubType(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isDynamicBanding(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.flowGroup(), buf);
  }
}



