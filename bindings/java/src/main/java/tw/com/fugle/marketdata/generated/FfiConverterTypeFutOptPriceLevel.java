package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptPriceLevel implements FfiConverterRustBuffer<FutOptPriceLevel> {
  INSTANCE;

  @Override
  public FutOptPriceLevel read(ByteBuffer buf) {
    return new FutOptPriceLevel(
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptPriceLevel value) {
      return (
            FfiConverterDouble.INSTANCE.allocationSize(value.price()) +
            FfiConverterLong.INSTANCE.allocationSize(value.size())
      );
  }

  @Override
  public void write(FutOptPriceLevel value, ByteBuffer buf) {
      FfiConverterDouble.INSTANCE.write(value.price(), buf);
      FfiConverterLong.INSTANCE.write(value.size(), buf);
  }
}



