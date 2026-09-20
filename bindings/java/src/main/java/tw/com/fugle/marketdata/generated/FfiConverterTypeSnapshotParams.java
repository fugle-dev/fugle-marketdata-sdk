package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeSnapshotParams implements FfiConverterRustBuffer<SnapshotParams> {
  INSTANCE;

  @Override
  public SnapshotParams read(ByteBuffer buf) {
    return new SnapshotParams(
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(SnapshotParams value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.typeFilter())
      );
  }

  @Override
  public void write(SnapshotParams value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.typeFilter(), buf);
  }
}



