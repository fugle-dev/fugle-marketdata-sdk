package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeInstitutionalTradesResponse implements FfiConverterRustBuffer<InstitutionalTradesResponse> {
  INSTANCE;

  @Override
  public InstitutionalTradesResponse read(ByteBuffer buf) {
    return new InstitutionalTradesResponse(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeInstitutionalTradesEntry.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(InstitutionalTradesResponse value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.market()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterSequenceTypeInstitutionalTradesEntry.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(InstitutionalTradesResponse value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.market(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterSequenceTypeInstitutionalTradesEntry.INSTANCE.write(value.data(), buf);
  }
}



