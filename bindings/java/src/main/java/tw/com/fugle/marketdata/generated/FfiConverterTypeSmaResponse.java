package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeSmaResponse implements FfiConverterRustBuffer<SmaResponse> {
  INSTANCE;

  @Override
  public SmaResponse read(ByteBuffer buf) {
    return new SmaResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterSequenceTypeSmaDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(SmaResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.period()) +
            FfiConverterSequenceTypeSmaDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(SmaResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterInteger.INSTANCE.write(value.period(), buf);
      FfiConverterSequenceTypeSmaDataPoint.INSTANCE.write(value.data(), buf);
  }
}



