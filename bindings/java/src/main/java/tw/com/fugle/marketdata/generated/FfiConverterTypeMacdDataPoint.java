package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMacdDataPoint implements FfiConverterRustBuffer<MacdDataPoint> {
  INSTANCE;

  @Override
  public MacdDataPoint read(ByteBuffer buf) {
    return new MacdDataPoint(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(MacdDataPoint value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.macd()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.signalValue()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.histogram())
      );
  }

  @Override
  public void write(MacdDataPoint value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.macd(), buf);
      FfiConverterDouble.INSTANCE.write(value.signalValue(), buf);
      FfiConverterDouble.INSTANCE.write(value.histogram(), buf);
  }
}



