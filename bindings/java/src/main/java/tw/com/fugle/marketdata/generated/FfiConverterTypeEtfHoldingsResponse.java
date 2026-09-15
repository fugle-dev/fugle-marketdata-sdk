package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeEtfHoldingsResponse implements FfiConverterRustBuffer<EtfHoldingsResponse> {
  INSTANCE;

  @Override
  public EtfHoldingsResponse read(ByteBuffer buf) {
    return new EtfHoldingsResponse(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeEtfHoldingsEntry.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(EtfHoldingsResponse value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterSequenceTypeEtfHoldingsEntry.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(EtfHoldingsResponse value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterSequenceTypeEtfHoldingsEntry.INSTANCE.write(value.data(), buf);
  }
}



