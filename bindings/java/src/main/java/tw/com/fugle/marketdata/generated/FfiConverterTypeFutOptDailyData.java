package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptDailyData implements FfiConverterRustBuffer<FutOptDailyData> {
  INSTANCE;

  @Override
  public FutOptDailyData read(ByteBuffer buf) {
    return new FutOptDailyData(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptDailyData value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.open()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.high()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.low()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.close()) +
            FfiConverterLong.INSTANCE.allocationSize(value.volume()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.openInterest()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.settlementPrice())
      );
  }

  @Override
  public void write(FutOptDailyData value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.open(), buf);
      FfiConverterDouble.INSTANCE.write(value.high(), buf);
      FfiConverterDouble.INSTANCE.write(value.low(), buf);
      FfiConverterDouble.INSTANCE.write(value.close(), buf);
      FfiConverterLong.INSTANCE.write(value.volume(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.openInterest(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.settlementPrice(), buf);
  }
}



