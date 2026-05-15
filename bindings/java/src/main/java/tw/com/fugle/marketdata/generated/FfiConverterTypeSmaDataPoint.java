package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeSmaDataPoint implements FfiConverterRustBuffer<SmaDataPoint> {
  INSTANCE;

  @Override
  public SmaDataPoint read(ByteBuffer buf) {
    return new SmaDataPoint(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(SmaDataPoint value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.sma())
      );
  }

  @Override
  public void write(SmaDataPoint value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.sma(), buf);
  }
}



