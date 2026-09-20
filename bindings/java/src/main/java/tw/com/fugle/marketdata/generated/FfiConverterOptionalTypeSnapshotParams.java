package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeSnapshotParams implements FfiConverterRustBuffer<SnapshotParams> {
  INSTANCE;

  @Override
  public SnapshotParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeSnapshotParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(SnapshotParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeSnapshotParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(SnapshotParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeSnapshotParams.INSTANCE.write(value, buf);
    }
  }
}



