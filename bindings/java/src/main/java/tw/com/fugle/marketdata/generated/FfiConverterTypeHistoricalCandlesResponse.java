package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeHistoricalCandlesResponse implements FfiConverterRustBuffer<HistoricalCandlesResponse> {
  INSTANCE;

  @Override
  public HistoricalCandlesResponse read(ByteBuffer buf) {
    return new HistoricalCandlesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterSequenceTypeHistoricalCandle.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(HistoricalCandlesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.adjusted()) +
            FfiConverterSequenceTypeHistoricalCandle.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(HistoricalCandlesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.adjusted(), buf);
      FfiConverterSequenceTypeHistoricalCandle.INSTANCE.write(value.data(), buf);
  }
}



