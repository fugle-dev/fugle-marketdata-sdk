package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeInstitutionalInvestorTrade implements FfiConverterRustBuffer<InstitutionalInvestorTrade> {
  INSTANCE;

  @Override
  public InstitutionalInvestorTrade read(ByteBuffer buf) {
    return new InstitutionalInvestorTrade(
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(InstitutionalInvestorTrade value) {
      return (
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.buy()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.sell()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.net())
      );
  }

  @Override
  public void write(InstitutionalInvestorTrade value, ByteBuffer buf) {
      FfiConverterOptionalDouble.INSTANCE.write(value.buy(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.sell(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.net(), buf);
  }
}



