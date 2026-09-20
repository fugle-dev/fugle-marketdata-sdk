package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptTradesParams implements FfiConverterRustBuffer<FutOptTradesParams> {
  INSTANCE;

  @Override
  public FutOptTradesParams read(ByteBuffer buf) {
    return new FutOptTradesParams(
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptTradesParams value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.offset()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.limit()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.isTrial())
      );
  }

  @Override
  public void write(FutOptTradesParams value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.offset(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.limit(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.isTrial(), buf);
  }
}



