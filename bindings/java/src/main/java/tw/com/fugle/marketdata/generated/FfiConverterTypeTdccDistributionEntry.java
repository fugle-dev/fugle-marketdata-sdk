package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeTdccDistributionEntry implements FfiConverterRustBuffer<TdccDistributionEntry> {
  INSTANCE;

  @Override
  public TdccDistributionEntry read(ByteBuffer buf) {
    return new TdccDistributionEntry(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeTdccDistributionLevel.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(TdccDistributionEntry value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterSequenceTypeTdccDistributionLevel.INSTANCE.allocationSize(value.distributions())
      );
  }

  @Override
  public void write(TdccDistributionEntry value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterSequenceTypeTdccDistributionLevel.INSTANCE.write(value.distributions(), buf);
  }
}



