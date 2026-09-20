package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeSubscribeOptions implements FfiConverterRustBuffer<SubscribeOptions> {
  INSTANCE;

  @Override
  public SubscribeOptions read(ByteBuffer buf) {
    return new SubscribeOptions(
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(SubscribeOptions value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.intradayOddLot())
      );
  }

  @Override
  public void write(SubscribeOptions value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.intradayOddLot(), buf);
  }
}



