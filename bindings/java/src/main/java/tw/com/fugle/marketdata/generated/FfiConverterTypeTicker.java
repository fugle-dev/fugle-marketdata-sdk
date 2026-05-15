package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTicker implements FfiConverterRustBuffer<Ticker> {
  INSTANCE;

  @Override
  public Ticker read(ByteBuffer buf) {
    return new Ticker(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalInteger.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(Ticker value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.nameEn()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.industry()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.securityType()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.referencePrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.limitUpPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.limitDownPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.previousClose()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.canDayTrade()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.canBuyDayTrade()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.canBelowFlatMarginShortSell()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.canBelowFlatSblShortSell()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isAttention()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isDisposition()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isUnusuallyRecommended()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isSpecificAbnormally()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.isNewlyCompiled()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.matchingInterval()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.securityStatus()) +
            FfiConverterOptionalInteger.INSTANCE.allocationSize(value.boardLot()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.tradingCurrency()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.exercisePrice()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.exercisedVolume()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.cancelledVolume()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.remainingVolume()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.exerciseRatio()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.capPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.floorPrice()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.maturityDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.openTime()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.closeTime())
      );
  }

  @Override
  public void write(Ticker value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.nameEn(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.industry(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.securityType(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.referencePrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.limitUpPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.limitDownPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.previousClose(), buf);
      FfiConverterBoolean.INSTANCE.write(value.canDayTrade(), buf);
      FfiConverterBoolean.INSTANCE.write(value.canBuyDayTrade(), buf);
      FfiConverterBoolean.INSTANCE.write(value.canBelowFlatMarginShortSell(), buf);
      FfiConverterBoolean.INSTANCE.write(value.canBelowFlatSblShortSell(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isAttention(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isDisposition(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isUnusuallyRecommended(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isSpecificAbnormally(), buf);
      FfiConverterBoolean.INSTANCE.write(value.isNewlyCompiled(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.matchingInterval(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.securityStatus(), buf);
      FfiConverterOptionalInteger.INSTANCE.write(value.boardLot(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.tradingCurrency(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.exercisePrice(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.exercisedVolume(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.cancelledVolume(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.remainingVolume(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.exerciseRatio(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.capPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.floorPrice(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.maturityDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.openTime(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.closeTime(), buf);
  }
}



