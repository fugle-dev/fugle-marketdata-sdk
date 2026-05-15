package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeQuote implements FfiConverterRustBuffer<Quote> {
  INSTANCE;

  @Override
  public Quote read(ByteBuffer buf) {
    return new Quote(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
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
      FfiConverterSequenceTypePriceLevel.INSTANCE.read(buf),
      FfiConverterSequenceTypePriceLevel.INSTANCE.read(buf),
      FfiConverterOptionalTypeTotalStats.INSTANCE.read(buf),
      FfiConverterOptionalTypeTradeInfo.INSTANCE.read(buf),
      FfiConverterOptionalTypeTradeInfo.INSTANCE.read(buf),
      FfiConverterOptionalTypeTradingHalt.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(Quote value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
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
            FfiConverterSequenceTypePriceLevel.INSTANCE.allocationSize(value.bids()) +
            FfiConverterSequenceTypePriceLevel.INSTANCE.allocationSize(value.asks()) +
            FfiConverterOptionalTypeTotalStats.INSTANCE.allocationSize(value.total()) +
            FfiConverterOptionalTypeTradeInfo.INSTANCE.allocationSize(value.lastTrade()) +
            FfiConverterOptionalTypeTradeInfo.INSTANCE.allocationSize(value.lastTrial()) +
            FfiConverterOptionalTypeTradingHalt.INSTANCE.allocationSize(value.tradingHalt()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitDownPrice()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitUpPrice()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitDownBid()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitUpBid()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitDownAsk()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitUpAsk()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitDownHalt()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isLimitUpHalt()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isTrial()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isDelayedOpen()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isDelayedClose()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isContinuous()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isOpen()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isClose()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.lastUpdated())
      );
  }

  @Override
  public void write(Quote value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
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
      FfiConverterSequenceTypePriceLevel.INSTANCE.write(value.bids(), buf);
      FfiConverterSequenceTypePriceLevel.INSTANCE.write(value.asks(), buf);
      FfiConverterOptionalTypeTotalStats.INSTANCE.write(value.total(), buf);
      FfiConverterOptionalTypeTradeInfo.INSTANCE.write(value.lastTrade(), buf);
      FfiConverterOptionalTypeTradeInfo.INSTANCE.write(value.lastTrial(), buf);
      FfiConverterOptionalTypeTradingHalt.INSTANCE.write(value.tradingHalt(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitDownPrice(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitUpPrice(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitDownBid(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitUpBid(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitDownAsk(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitUpAsk(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitDownHalt(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isLimitUpHalt(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isTrial(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isDelayedOpen(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isDelayedClose(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isContinuous(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isOpen(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isClose(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.lastUpdated(), buf);
  }
}



