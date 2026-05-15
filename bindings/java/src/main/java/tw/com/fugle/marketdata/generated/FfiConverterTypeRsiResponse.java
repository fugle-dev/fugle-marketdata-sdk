package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeRsiResponse implements FfiConverterRustBuffer<RsiResponse> {
  INSTANCE;

  @Override
  public RsiResponse read(ByteBuffer buf) {
    return new RsiResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterSequenceTypeRsiDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(RsiResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.period()) +
            FfiConverterSequenceTypeRsiDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(RsiResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterInteger.INSTANCE.write(value.period(), buf);
      FfiConverterSequenceTypeRsiDataPoint.INSTANCE.write(value.data(), buf);
  }
}



