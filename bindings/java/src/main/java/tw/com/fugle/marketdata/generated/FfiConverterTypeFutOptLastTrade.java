package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptLastTrade implements FfiConverterRustBuffer<FutOptLastTrade> {
  INSTANCE;

  @Override
  public FutOptLastTrade read(ByteBuffer buf) {
    return new FutOptLastTrade(
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptLastTrade value) {
      return (
            FfiConverterDouble.INSTANCE.allocationSize(value.price()) +
            FfiConverterLong.INSTANCE.allocationSize(value.size()) +
            FfiConverterLong.INSTANCE.allocationSize(value.time())
      );
  }

  @Override
  public void write(FutOptLastTrade value, ByteBuffer buf) {
      FfiConverterDouble.INSTANCE.write(value.price(), buf);
      FfiConverterLong.INSTANCE.write(value.size(), buf);
      FfiConverterLong.INSTANCE.write(value.time(), buf);
  }
}



