package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTradingHalt implements FfiConverterRustBuffer<TradingHalt> {
  INSTANCE;

  @Override
  public TradingHalt read(ByteBuffer buf) {
    return new TradingHalt(
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TradingHalt value) {
      return (
            FfiConverterBoolean.INSTANCE.allocationSize(value.isHalted()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.time())
      );
  }

  @Override
  public void write(TradingHalt value, ByteBuffer buf) {
      FfiConverterBoolean.INSTANCE.write(value.isHalted(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.time(), buf);
  }
}



