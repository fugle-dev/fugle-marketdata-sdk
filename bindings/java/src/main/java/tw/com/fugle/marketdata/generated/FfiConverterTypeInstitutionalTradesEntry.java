package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeInstitutionalTradesEntry implements FfiConverterRustBuffer<InstitutionalTradesEntry> {
  INSTANCE;

  @Override
  public InstitutionalTradesEntry read(ByteBuffer buf) {
    return new InstitutionalTradesEntry(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.read(buf),
      FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.read(buf),
      FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(InstitutionalTradesEntry value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.allocationSize(value.foreign()) +
            FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.allocationSize(value.trust()) +
            FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.allocationSize(value.dealer()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.total())
      );
  }

  @Override
  public void write(InstitutionalTradesEntry value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.write(value.foreign(), buf);
      FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.write(value.trust(), buf);
      FfiConverterOptionalTypeInstitutionalInvestorTrade.INSTANCE.write(value.dealer(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.total(), buf);
  }
}



