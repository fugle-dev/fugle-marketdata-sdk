package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTdccDistributionLevel implements FfiConverterRustBuffer<TdccDistributionLevel> {
  INSTANCE;

  @Override
  public TdccDistributionLevel read(ByteBuffer buf) {
    return new TdccDistributionLevel(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TdccDistributionLevel value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.range()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.holders()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.shares()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.proportion())
      );
  }

  @Override
  public void write(TdccDistributionLevel value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.range(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.holders(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.shares(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.proportion(), buf);
  }
}



