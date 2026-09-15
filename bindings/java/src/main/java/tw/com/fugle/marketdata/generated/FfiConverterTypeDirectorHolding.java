package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDirectorHolding implements FfiConverterRustBuffer<DirectorHolding> {
  INSTANCE;

  @Override
  public DirectorHolding read(ByteBuffer buf) {
    return new DirectorHolding(
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf),
      FfiConverterOptionalDouble.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(DirectorHolding value) {
      return (
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.order()) +
            FfiConverterString.INSTANCE.allocationSize(value.title()) +
            FfiConverterString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.electedShares()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.heldShares()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.pledgedShares()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.pledgeRatio()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.relatedHeldShares()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.relatedPledgedShares()) +
            FfiConverterOptionalDouble.INSTANCE.allocationSize(value.relatedPledgeRatio())
      );
  }

  @Override
  public void write(DirectorHolding value, ByteBuffer buf) {
      FfiConverterOptionalLong.INSTANCE.write(value.order(), buf);
      FfiConverterString.INSTANCE.write(value.title(), buf);
      FfiConverterString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.electedShares(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.heldShares(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.pledgedShares(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.pledgeRatio(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.relatedHeldShares(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.relatedPledgedShares(), buf);
      FfiConverterOptionalDouble.INSTANCE.write(value.relatedPledgeRatio(), buf);
  }
}



