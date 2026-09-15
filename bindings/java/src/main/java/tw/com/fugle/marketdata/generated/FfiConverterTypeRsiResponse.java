package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeRsiResponse implements FfiConverterRustBuffer<RsiResponse> {
  INSTANCE;

  @Override
  public RsiResponse read(ByteBuffer buf) {
    return new RsiResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterSequenceTypeRsiDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(RsiResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.period()) +
            FfiConverterSequenceTypeRsiDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(RsiResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterInteger.INSTANCE.write(value.period(), buf);
      FfiConverterSequenceTypeRsiDataPoint.INSTANCE.write(value.data(), buf);
  }
}



