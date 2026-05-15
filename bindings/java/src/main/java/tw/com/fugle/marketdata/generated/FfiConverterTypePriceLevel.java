package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypePriceLevel implements FfiConverterRustBuffer<PriceLevel> {
  INSTANCE;

  @Override
  public PriceLevel read(ByteBuffer buf) {
    return new PriceLevel(
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(PriceLevel value) {
      return (
            FfiConverterDouble.INSTANCE.allocationSize(value.price()) +
            FfiConverterLong.INSTANCE.allocationSize(value.size())
      );
  }

  @Override
  public void write(PriceLevel value, ByteBuffer buf) {
      FfiConverterDouble.INSTANCE.write(value.price(), buf);
      FfiConverterLong.INSTANCE.write(value.size(), buf);
  }
}



