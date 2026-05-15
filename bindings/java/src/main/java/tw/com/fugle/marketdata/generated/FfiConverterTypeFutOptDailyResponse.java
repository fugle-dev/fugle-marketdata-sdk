package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptDailyResponse implements FfiConverterRustBuffer<FutOptDailyResponse> {
  INSTANCE;

  @Override
  public FutOptDailyResponse read(ByteBuffer buf) {
    return new FutOptDailyResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterSequenceTypeFutOptDailyData.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptDailyResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterSequenceTypeFutOptDailyData.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(FutOptDailyResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterSequenceTypeFutOptDailyData.INSTANCE.write(value.data(), buf);
  }
}



