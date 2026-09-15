package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeStreamingVersionRecord implements FfiConverterRustBuffer<StreamingVersionRecord> {
  INSTANCE;

  @Override
  public StreamingVersionRecord read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeStreamingVersionRecord.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(StreamingVersionRecord value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeStreamingVersionRecord.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(StreamingVersionRecord value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeStreamingVersionRecord.INSTANCE.write(value, buf);
    }
  }
}



