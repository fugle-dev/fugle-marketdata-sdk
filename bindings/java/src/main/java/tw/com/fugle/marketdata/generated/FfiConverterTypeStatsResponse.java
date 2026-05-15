package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeStatsResponse implements FfiConverterRustBuffer<StatsResponse> {
  INSTANCE;

  @Override
  public StatsResponse read(ByteBuffer buf) {
    return new StatsResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(StatsResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterString.INSTANCE.allocationSize(value.name()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.openPrice()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.highPrice()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.lowPrice()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.closePrice()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.change()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.changePercent()) +
            FfiConverterLong.INSTANCE.allocationSize(value.tradeVolume()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.tradeValue()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.previousClose()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.week52High()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.week52Low())
      );
  }

  @Override
  public void write(StatsResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterString.INSTANCE.write(value.name(), buf);
      FfiConverterDouble.INSTANCE.write(value.openPrice(), buf);
      FfiConverterDouble.INSTANCE.write(value.highPrice(), buf);
      FfiConverterDouble.INSTANCE.write(value.lowPrice(), buf);
      FfiConverterDouble.INSTANCE.write(value.closePrice(), buf);
      FfiConverterDouble.INSTANCE.write(value.change(), buf);
      FfiConverterDouble.INSTANCE.write(value.changePercent(), buf);
      FfiConverterLong.INSTANCE.write(value.tradeVolume(), buf);
      FfiConverterDouble.INSTANCE.write(value.tradeValue(), buf);
      FfiConverterDouble.INSTANCE.write(value.previousClose(), buf);
      FfiConverterDouble.INSTANCE.write(value.week52High(), buf);
      FfiConverterDouble.INSTANCE.write(value.week52Low(), buf);
  }
}



