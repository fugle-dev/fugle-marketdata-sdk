package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStreamMessage implements FfiConverterRustBuffer<StreamMessage> {
  INSTANCE;

  @Override
  public StreamMessage read(ByteBuffer buf) {
    return new StreamMessage(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StreamMessage value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.event()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.channel()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.id()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataJson()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.errorCode()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.errorMessage())
      );
  }

  @Override
  public void write(StreamMessage value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.event(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.channel(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.id(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataJson(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.errorCode(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.errorMessage(), buf);
  }
}



