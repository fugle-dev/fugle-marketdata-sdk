package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDirectorHoldingsEntry implements FfiConverterRustBuffer<DirectorHoldingsEntry> {
  INSTANCE;

  @Override
  public DirectorHoldingsEntry read(ByteBuffer buf) {
    return new DirectorHoldingsEntry(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeDirectorHolding.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(DirectorHoldingsEntry value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterSequenceTypeDirectorHolding.INSTANCE.allocationSize(value.directors())
      );
  }

  @Override
  public void write(DirectorHoldingsEntry value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterSequenceTypeDirectorHolding.INSTANCE.write(value.directors(), buf);
  }
}



