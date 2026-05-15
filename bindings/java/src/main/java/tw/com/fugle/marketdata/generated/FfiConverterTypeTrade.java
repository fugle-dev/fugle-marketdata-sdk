package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTrade implements FfiConverterRustBuffer<Trade> {
  INSTANCE;

  @Override
  public Trade read(ByteBuffer buf) {
    return new Trade(
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(Trade value) {
      return (
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.bid()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.ask()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.price()) +
            FfiConverterLong.INSTANCE.allocationSize(value.size()) +
            FfiConverterLong.INSTANCE.allocationSize(value.time())
      );
  }

  @Override
  public void write(Trade value, ByteBuffer buf) {
      FfiConverterOptionalDouble.INSTANCE.write(value.bid(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.ask(), buf);
      FfiConverterDouble.INSTANCE.write(value.price(), buf);
      FfiConverterLong.INSTANCE.write(value.size(), buf);
      FfiConverterLong.INSTANCE.write(value.time(), buf);
  }
}



