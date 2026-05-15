package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTradesResponse implements FfiConverterRustBuffer<TradesResponse> {
  INSTANCE;

  @Override
  public TradesResponse read(ByteBuffer buf) {
    return new TradesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeTrade.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TradesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterSequenceTypeTrade.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(TradesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterSequenceTypeTrade.INSTANCE.write(value.data(), buf);
  }
}



