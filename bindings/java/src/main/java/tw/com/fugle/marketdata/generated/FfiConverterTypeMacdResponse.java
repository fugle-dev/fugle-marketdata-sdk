package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMacdResponse implements FfiConverterRustBuffer<MacdResponse> {
  INSTANCE;

  @Override
  public MacdResponse read(ByteBuffer buf) {
    return new MacdResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterInteger.INSTANCE.read(buf),
      FfiConverterSequenceTypeMacdDataPoint.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(MacdResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.timeframe()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.fast()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.slow()) +
            FfiConverterInteger.INSTANCE.allocationSize(value.signal()) +
            FfiConverterSequenceTypeMacdDataPoint.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(MacdResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.timeframe(), buf);
      FfiConverterInteger.INSTANCE.write(value.fast(), buf);
      FfiConverterInteger.INSTANCE.write(value.slow(), buf);
      FfiConverterInteger.INSTANCE.write(value.signal(), buf);
      FfiConverterSequenceTypeMacdDataPoint.INSTANCE.write(value.data(), buf);
  }
}



