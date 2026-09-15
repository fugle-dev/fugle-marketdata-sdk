package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeEtfHoldingsEntry implements FfiConverterRustBuffer<EtfHoldingsEntry> {
  INSTANCE;

  @Override
  public EtfHoldingsEntry read(ByteBuffer buf) {
    return new EtfHoldingsEntry(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeEtfHoldingComponent.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(EtfHoldingsEntry value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterSequenceTypeEtfHoldingComponent.INSTANCE.allocationSize(value.components())
      );
  }

  @Override
  public void write(EtfHoldingsEntry value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterSequenceTypeEtfHoldingComponent.INSTANCE.write(value.components(), buf);
  }
}



