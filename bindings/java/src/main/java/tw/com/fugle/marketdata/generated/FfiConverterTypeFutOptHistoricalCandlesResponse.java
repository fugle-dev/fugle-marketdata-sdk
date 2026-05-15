package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptHistoricalCandlesResponse implements FfiConverterRustBuffer<FutOptHistoricalCandlesResponse> {
  INSTANCE;

  @Override
  public FutOptHistoricalCandlesResponse read(ByteBuffer buf) {
    return new FutOptHistoricalCandlesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterSequenceTypeFutOptHistoricalCandle.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptHistoricalCandlesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterSequenceTypeFutOptHistoricalCandle.INSTANCE.allocationSize(value.candles())
      );
  }

  @Override
  public void write(FutOptHistoricalCandlesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterSequenceTypeFutOptHistoricalCandle.INSTANCE.write(value.candles(), buf);
  }
}



