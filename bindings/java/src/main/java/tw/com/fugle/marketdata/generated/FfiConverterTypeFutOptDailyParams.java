package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptDailyParams implements FfiConverterRustBuffer<FutOptDailyParams> {
  INSTANCE;

  @Override
  public FutOptDailyParams read(ByteBuffer buf) {
    return new FutOptDailyParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptDailyParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours())
      );
  }

  @Override
  public void write(FutOptDailyParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
  }
}



