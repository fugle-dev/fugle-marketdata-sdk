package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeBbDataPoint implements FfiConverterRustBuffer<BbDataPoint> {
  INSTANCE;

  @Override
  public BbDataPoint read(ByteBuffer buf) {
    return new BbDataPoint(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(BbDataPoint value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.upper()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.middle()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.lower())
      );
  }

  @Override
  public void write(BbDataPoint value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.upper(), buf);
      FfiConverterDouble.INSTANCE.write(value.middle(), buf);
      FfiConverterDouble.INSTANCE.write(value.lower(), buf);
  }
}



