package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeSmaResponse implements FfiConverterRustBuffer<SmaResponse> {
  INSTANCE;

  @Override
  public SmaResponse read(ByteBuffer buf) {
    return new SmaResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterSequenceTypeSmaDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(SmaResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.period()) +
            FfiConverterSequenceTypeSmaDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(SmaResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterInteger.INSTANCE.write(value.period(), buf);
      FfiConverterSequenceTypeSmaDataPoint.INSTANCE.write(value.data(), buf);
  }
}



