package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeIntradayCandle implements FfiConverterRustBuffer<IntradayCandle> {
  INSTANCE;

  @Override
  public IntradayCandle read(ByteBuffer buf) {
    return new IntradayCandle(
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(IntradayCandle value) {
      return (
            FfiConverterDouble.INSTANCE.allocationSize(value.open()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.high()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.low()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.close()) +
            FfiConverterLong.INSTANCE.allocationSize(value.volume()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.average()) +
            FfiConverterString.INSTANCE.allocationSize(value.date())
      );
  }

  @Override
  public void write(IntradayCandle value, ByteBuffer buf) {
      FfiConverterDouble.INSTANCE.write(value.open(), buf);
      FfiConverterDouble.INSTANCE.write(value.high(), buf);
      FfiConverterDouble.INSTANCE.write(value.low(), buf);
      FfiConverterDouble.INSTANCE.write(value.close(), buf);
      FfiConverterLong.INSTANCE.write(value.volume(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.average(), buf);
      FfiConverterString.INSTANCE.write(value.date(), buf);
  }
}



