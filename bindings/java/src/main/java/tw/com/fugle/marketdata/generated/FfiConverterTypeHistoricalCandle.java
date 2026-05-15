package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeHistoricalCandle implements FfiConverterRustBuffer<HistoricalCandle> {
  INSTANCE;

  @Override
  public HistoricalCandle read(ByteBuffer buf) {
    return new HistoricalCandle(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(HistoricalCandle value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.open()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.high()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.low()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.close()) +
            FfiConverterLong.INSTANCE.allocationSize(value.volume()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.turnover()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.change())
      );
  }

  @Override
  public void write(HistoricalCandle value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.open(), buf);
      FfiConverterDouble.INSTANCE.write(value.high(), buf);
      FfiConverterDouble.INSTANCE.write(value.low(), buf);
      FfiConverterDouble.INSTANCE.write(value.close(), buf);
      FfiConverterLong.INSTANCE.write(value.volume(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.turnover(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.change(), buf);
  }
}



