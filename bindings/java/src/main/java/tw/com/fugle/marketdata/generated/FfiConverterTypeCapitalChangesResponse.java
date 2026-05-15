package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeCapitalChangesResponse implements FfiConverterRustBuffer<CapitalChangesResponse> {
  INSTANCE;

  @Override
  public CapitalChangesResponse read(ByteBuffer buf) {
    return new CapitalChangesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeCapitalChange.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(CapitalChangesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterSequenceTypeCapitalChange.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(CapitalChangesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterSequenceTypeCapitalChange.INSTANCE.write(value.data(), buf);
  }
}



