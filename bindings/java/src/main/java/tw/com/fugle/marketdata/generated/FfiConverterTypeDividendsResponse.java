package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDividendsResponse implements FfiConverterRustBuffer<DividendsResponse> {
  INSTANCE;

  @Override
  public DividendsResponse read(ByteBuffer buf) {
    return new DividendsResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeDividend.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(DividendsResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterSequenceTypeDividend.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(DividendsResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterSequenceTypeDividend.INSTANCE.write(value.data(), buf);
  }
}



