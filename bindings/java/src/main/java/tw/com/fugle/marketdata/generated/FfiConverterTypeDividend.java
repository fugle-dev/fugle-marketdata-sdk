package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDividend implements FfiConverterRustBuffer<Dividend> {
  INSTANCE;

  @Override
  public Dividend read(ByteBuffer buf) {
    return new Dividend(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(Dividend value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.exDividendDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.paymentDate()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.cashDividend()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.stockDividend()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dividendYear())
      );
  }

  @Override
  public void write(Dividend value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.exDividendDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.paymentDate(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.cashDividend(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.stockDividend(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.dividendYear(), buf);
  }
}



