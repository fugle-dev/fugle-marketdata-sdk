package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeKdjResponse implements FfiConverterRustBuffer<KdjResponse> {
  INSTANCE;

  @Override
  public KdjResponse read(ByteBuffer buf) {
    return new KdjResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterSequenceTypeKdjDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(KdjResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.rPeriod()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.kPeriod()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.dPeriod()) +
            FfiConverterSequenceTypeKdjDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(KdjResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.rPeriod(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.kPeriod(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.dPeriod(), buf);
      FfiConverterSequenceTypeKdjDataPoint.INSTANCE.write(value.data(), buf);
  }
}



