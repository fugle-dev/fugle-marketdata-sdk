package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTotalStats implements FfiConverterRustBuffer<TotalStats> {
  INSTANCE;

  @Override
  public TotalStats read(ByteBuffer buf) {
    return new TotalStats(
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TotalStats value) {
      return (
            FfiConverterDouble.INSTANCE.allocationSize(value.tradeValue()) +
            FfiConverterLong.INSTANCE.allocationSize(value.tradeVolume()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.tradeVolumeAtBid()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.tradeVolumeAtAsk()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.transaction()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.time())
      );
  }

  @Override
  public void write(TotalStats value, ByteBuffer buf) {
      FfiConverterDouble.INSTANCE.write(value.tradeValue(), buf);
      FfiConverterLong.INSTANCE.write(value.tradeVolume(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.tradeVolumeAtBid(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.tradeVolumeAtAsk(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.transaction(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.time(), buf);
  }
}



