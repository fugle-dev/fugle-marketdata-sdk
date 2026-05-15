package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMover implements FfiConverterRustBuffer<Mover> {
  INSTANCE;

  @Override
  public Mover read(ByteBuffer buf) {
    return new Mover(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(Mover value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.openPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.highPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.lowPrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.closePrice()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.change()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.changePercent()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.tradeVolume()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.tradeValue()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.lastUpdated())
      );
  }

  @Override
  public void write(Mover value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.openPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.highPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.lowPrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.closePrice(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.change(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.changePercent(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.tradeVolume(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.tradeValue(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.lastUpdated(), buf);
  }
}



