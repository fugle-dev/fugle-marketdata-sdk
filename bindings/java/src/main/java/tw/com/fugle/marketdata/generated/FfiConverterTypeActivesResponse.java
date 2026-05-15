package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeActivesResponse implements FfiConverterRustBuffer<ActivesResponse> {
  INSTANCE;

  @Override
  public ActivesResponse read(ByteBuffer buf) {
    return new ActivesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeActive.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ActivesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterString.INSTANCE.allocationSize(value.time()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterSequenceTypeActive.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(ActivesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterString.INSTANCE.write(value.time(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterSequenceTypeActive.INSTANCE.write(value.data(), buf);
  }
}



