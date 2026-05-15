package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptTotalStats implements FfiConverterRustBuffer<FutOptTotalStats> {
  INSTANCE;

  @Override
  public FutOptTotalStats read(ByteBuffer buf) {
    return new FutOptTotalStats(
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptTotalStats value) {
      return (
            FfiConverterLong.INSTANCE.allocationSize(value.tradeVolume()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.totalBidMatch()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.totalAskMatch())
      );
  }

  @Override
  public void write(FutOptTotalStats value, ByteBuffer buf) {
      FfiConverterLong.INSTANCE.write(value.tradeVolume(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.totalBidMatch(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.totalAskMatch(), buf);
  }
}



