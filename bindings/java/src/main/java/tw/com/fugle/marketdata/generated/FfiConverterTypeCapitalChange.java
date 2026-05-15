package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeCapitalChange implements FfiConverterRustBuffer<CapitalChange> {
  INSTANCE;

  @Override
  public CapitalChange read(ByteBuffer buf) {
    return new CapitalChange(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(CapitalChange value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.previousCapital()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.currentCapital()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.changeType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.reason())
      );
  }

  @Override
  public void write(CapitalChange value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.previousCapital(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.currentCapital(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.changeType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.reason(), buf);
  }
}



