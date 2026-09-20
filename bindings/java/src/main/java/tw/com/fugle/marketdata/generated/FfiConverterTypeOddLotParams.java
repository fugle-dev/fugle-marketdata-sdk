package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeOddLotParams implements FfiConverterRustBuffer<OddLotParams> {
  INSTANCE;

  @Override
  public OddLotParams read(ByteBuffer buf) {
    return new OddLotParams(
      FfiConverterOptionalBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(OddLotParams value) {
      return (
            FfiConverterOptionalBoolean.INSTANCE.allocationSize(value.oddLot())
      );
  }

  @Override
  public void write(OddLotParams value, ByteBuffer buf) {
      FfiConverterOptionalBoolean.INSTANCE.write(value.oddLot(), buf);
  }
}



