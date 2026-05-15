package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeIntradayCandlesResponse implements FfiConverterRustBuffer<IntradayCandlesResponse> {
  INSTANCE;

  @Override
  public IntradayCandlesResponse read(ByteBuffer buf) {
    return new IntradayCandlesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterSequenceTypeIntradayCandle.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(IntradayCandlesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterSequenceTypeIntradayCandle.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(IntradayCandlesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterSequenceTypeIntradayCandle.INSTANCE.write(value.data(), buf);
  }
}



