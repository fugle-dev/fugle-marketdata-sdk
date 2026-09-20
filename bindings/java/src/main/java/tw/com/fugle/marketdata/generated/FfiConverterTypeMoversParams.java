package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMoversParams implements FfiConverterRustBuffer<MoversParams> {
  INSTANCE;

  @Override
  public MoversParams read(ByteBuffer buf) {
    return new MoversParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(MoversParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.typeFilter()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.gt()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.gte()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.lt()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.lte()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.eq())
      );
  }

  @Override
  public void write(MoversParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.typeFilter(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.gt(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.gte(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.lt(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.lte(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.eq(), buf);
  }
}



