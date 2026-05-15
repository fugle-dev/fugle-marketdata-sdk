package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeKdjDataPoint implements FfiConverterRustBuffer<KdjDataPoint> {
  INSTANCE;

  @Override
  public KdjDataPoint read(ByteBuffer buf) {
    return new KdjDataPoint(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(KdjDataPoint value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.k()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.d()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.j())
      );
  }

  @Override
  public void write(KdjDataPoint value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterDouble.INSTANCE.write(value.k(), buf);
      FfiConverterDouble.INSTANCE.write(value.d(), buf);
      FfiConverterDouble.INSTANCE.write(value.j(), buf);
  }
}



