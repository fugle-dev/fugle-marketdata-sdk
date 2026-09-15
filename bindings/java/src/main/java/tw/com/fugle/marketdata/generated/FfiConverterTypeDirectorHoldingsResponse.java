package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDirectorHoldingsResponse implements FfiConverterRustBuffer<DirectorHoldingsResponse> {
  INSTANCE;

  @Override
  public DirectorHoldingsResponse read(ByteBuffer buf) {
    return new DirectorHoldingsResponse(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeDirectorHoldingsEntry.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(DirectorHoldingsResponse value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterSequenceTypeDirectorHoldingsEntry.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(DirectorHoldingsResponse value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterSequenceTypeDirectorHoldingsEntry.INSTANCE.write(value.data(), buf);
  }
}



