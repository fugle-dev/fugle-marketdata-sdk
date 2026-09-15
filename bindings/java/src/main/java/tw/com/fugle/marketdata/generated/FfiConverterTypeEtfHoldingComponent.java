package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeEtfHoldingComponent implements FfiConverterRustBuffer<EtfHoldingComponent> {
  INSTANCE;

  @Override
  public EtfHoldingComponent read(ByteBuffer buf) {
    return new EtfHoldingComponent(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(EtfHoldingComponent value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterString.INSTANCE.allocationSize(value.name()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.quantity()) +
            FfiConverterDouble.INSTANCE.allocationSize(value.weight()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.quantityChange()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.weightChange())
      );
  }

  @Override
  public void write(EtfHoldingComponent value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterString.INSTANCE.write(value.name(), buf);
      FfiConverterDouble.INSTANCE.write(value.quantity(), buf);
      FfiConverterDouble.INSTANCE.write(value.weight(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.quantityChange(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.weightChange(), buf);
  }
}



