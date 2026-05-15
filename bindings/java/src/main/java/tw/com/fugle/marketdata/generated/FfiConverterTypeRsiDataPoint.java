package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeRsiDataPoint implements FfiConverterRustBuffer<RsiDataPoint> {
  INSTANCE;

  @Override
  public RsiDataPoint read(ByteBuffer buf) {
    return new RsiDataPoint(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(RsiDataPoint value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.rsi())
      );
  }

  @Override
  public void write(RsiDataPoint value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.rsi(), buf);
  }
}



