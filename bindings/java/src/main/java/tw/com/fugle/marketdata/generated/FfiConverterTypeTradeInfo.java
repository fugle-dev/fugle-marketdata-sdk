package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTradeInfo implements FfiConverterRustBuffer<TradeInfo> {
  INSTANCE;

  @Override
  public TradeInfo read(ByteBuffer buf) {
    return new TradeInfo(
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TradeInfo value) {
      return (
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.bid()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.ask()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.price()) +
            FfiConverterLong.INSTANCE.allocationSize(value.size()) +
            FfiConverterLong.INSTANCE.allocationSize(value.time())
      );
  }

  @Override
  public void write(TradeInfo value, ByteBuffer buf) {
      FfiConverterOptionalDouble.INSTANCE.write(value.bid(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.ask(), buf);
      FfiConverterDouble.INSTANCE.write(value.price(), buf);
      FfiConverterLong.INSTANCE.write(value.size(), buf);
      FfiConverterLong.INSTANCE.write(value.time(), buf);
  }
}



