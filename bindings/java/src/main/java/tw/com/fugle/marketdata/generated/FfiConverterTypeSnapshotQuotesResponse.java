package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeSnapshotQuotesResponse implements FfiConverterRustBuffer<SnapshotQuotesResponse> {
  INSTANCE;

  @Override
  public SnapshotQuotesResponse read(ByteBuffer buf) {
    return new SnapshotQuotesResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeSnapshotQuote.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(SnapshotQuotesResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.date()) +
            FfiConverterString.INSTANCE.allocationSize(value.time()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterSequenceTypeSnapshotQuote.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(SnapshotQuotesResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.date(), buf);
      FfiConverterString.INSTANCE.write(value.time(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterSequenceTypeSnapshotQuote.INSTANCE.write(value.data(), buf);
  }
}



