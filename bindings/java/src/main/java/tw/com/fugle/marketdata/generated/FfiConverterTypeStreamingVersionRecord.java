package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStreamingVersionRecord implements FfiConverterRustBuffer<StreamingVersionRecord> {
  INSTANCE;

  @Override
  public StreamingVersionRecord read(ByteBuffer buf) {
    return new StreamingVersionRecord(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StreamingVersionRecord value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.stock()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.futopt())
      );
  }

  @Override
  public void write(StreamingVersionRecord value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.stock(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.futopt(), buf);
  }
}



