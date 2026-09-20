package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeCorporateActionsParams implements FfiConverterRustBuffer<CorporateActionsParams> {
  INSTANCE;

  @Override
  public CorporateActionsParams read(ByteBuffer buf) {
    return new CorporateActionsParams(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(CorporateActionsParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.startDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.endDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.sort())
      );
  }

  @Override
  public void write(CorporateActionsParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.startDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.endDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exchange(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.sort(), buf);
  }
}



