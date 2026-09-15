package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTdccDistributionResponse implements FfiConverterRustBuffer<TdccDistributionResponse> {
  INSTANCE;

  @Override
  public TdccDistributionResponse read(ByteBuffer buf) {
    return new TdccDistributionResponse(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeTdccDistributionEntry.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TdccDistributionResponse value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterSequenceTypeTdccDistributionEntry.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(TdccDistributionResponse value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterSequenceTypeTdccDistributionEntry.INSTANCE.write(value.data(), buf);
  }
}



