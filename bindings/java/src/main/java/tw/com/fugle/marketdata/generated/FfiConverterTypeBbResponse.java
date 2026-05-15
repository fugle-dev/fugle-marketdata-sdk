package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeBbResponse implements FfiConverterRustBuffer<BbResponse> {
  INSTANCE;

  @Override
  public BbResponse read(ByteBuffer buf) {
    return new BbResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterSequenceTypeBbDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(BbResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.period()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.stddev()) +
            FfiConverterSequenceTypeBbDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(BbResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterInteger.INSTANCE.write(value.period(), buf);
      FfiConverterDouble.INSTANCE.write(value.stddev(), buf);
      FfiConverterSequenceTypeBbDataPoint.INSTANCE.write(value.data(), buf);
  }
}



