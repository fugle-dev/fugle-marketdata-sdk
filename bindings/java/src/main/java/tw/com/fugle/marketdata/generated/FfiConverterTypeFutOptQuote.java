package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeFutOptQuote implements FfiConverterRustBuffer<FutOptQuote> {
  INSTANCE;

  @Override
  public FutOptQuote read(ByteBuffer buf) {
    return new FutOptQuote(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterSequenceTypeFutOptPriceLevel.INSTANCE.read(buf),
      FfiConverterSequenceTypeFutOptPriceLevel.INSTANCE.read(buf),
      FfiConverterOptionalTypeFutOptTotalStats.INSTANCE.read(buf),
      FfiConverterOptionalTypeFutOptLastTrade.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(FutOptQuote value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.previousClose()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.openPrice()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.openTime()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.highPrice()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.highTime()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.lowPrice()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.lowTime()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.closePrice()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.closeTime()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.lastPrice()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.lastSize()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.avgPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.change()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.changePercent()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.amplitude()) +
            FfiConverterSequenceTypeFutOptPriceLevel.INSTANCE.allocationSize(value.bids()) +
            FfiConverterSequenceTypeFutOptPriceLevel.INSTANCE.allocationSize(value.asks()) +
            FfiConverterOptionalTypeFutOptTotalStats.INSTANCE.allocationSize(value.total()) +
            FfiConverterOptionalTypeFutOptLastTrade.INSTANCE.allocationSize(value.lastTrade()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.lastUpdated())
      );
  }

  @Override
  public void write(FutOptQuote value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.previousClose(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.openPrice(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.openTime(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.highPrice(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.highTime(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.lowPrice(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.lowTime(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.closePrice(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.closeTime(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.lastPrice(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.lastSize(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.avgPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.change(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.changePercent(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.amplitude(), buf);
      FfiConverterSequenceTypeFutOptPriceLevel.INSTANCE.write(value.bids(), buf);
      FfiConverterSequenceTypeFutOptPriceLevel.INSTANCE.write(value.asks(), buf);
      FfiConverterOptionalTypeFutOptTotalStats.INSTANCE.write(value.total(), buf);
      FfiConverterOptionalTypeFutOptLastTrade.INSTANCE.write(value.lastTrade(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.lastUpdated(), buf);
  }
}



