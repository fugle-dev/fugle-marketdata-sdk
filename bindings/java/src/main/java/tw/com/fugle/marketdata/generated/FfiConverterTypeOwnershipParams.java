package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeOwnershipParams implements FfiConverterRustBuffer<OwnershipParams> {
  INSTANCE;

  @Override
  public OwnershipParams read(ByteBuffer buf) {
    return new OwnershipParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(OwnershipParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.from()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.to()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sort())
      );
  }

  @Override
  public void write(OwnershipParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.from(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.to(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sort(), buf);
  }
}



