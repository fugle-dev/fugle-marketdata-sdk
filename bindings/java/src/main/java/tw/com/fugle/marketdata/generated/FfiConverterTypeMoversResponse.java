package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMoversResponse implements FfiConverterRustBuffer<MoversResponse> {
  INSTANCE;

  @Override
  public MoversResponse read(ByteBuffer buf) {
    return new MoversResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeMover.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(MoversResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterString.INSTANCE.allocationSize(value.time()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterSequenceTypeMover.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(MoversResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterString.INSTANCE.write(value.time(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterSequenceTypeMover.INSTANCE.write(value.data(), buf);
  }
}



