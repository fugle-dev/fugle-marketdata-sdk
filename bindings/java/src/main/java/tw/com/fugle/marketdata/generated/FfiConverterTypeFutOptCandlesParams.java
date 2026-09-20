package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptCandlesParams implements FfiConverterRustBuffer<FutOptCandlesParams> {
  INSTANCE;

  @Override
  public FutOptCandlesParams read(ByteBuffer buf) {
    return new FutOptCandlesParams(
      FfiConverterOptionalBoolean.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptCandlesParams value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.timeframe())
      );
  }

  @Override
  public void write(FutOptCandlesParams value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.timeframe(), buf);
  }
}



