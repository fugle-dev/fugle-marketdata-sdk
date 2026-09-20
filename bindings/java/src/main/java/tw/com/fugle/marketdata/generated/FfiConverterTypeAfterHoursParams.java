package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeAfterHoursParams implements FfiConverterRustBuffer<AfterHoursParams> {
  INSTANCE;

  @Override
  public AfterHoursParams read(ByteBuffer buf) {
    return new AfterHoursParams(
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(AfterHoursParams value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.afterHours())
      );
  }

  @Override
  public void write(AfterHoursParams value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.afterHours(), buf);
  }
}



