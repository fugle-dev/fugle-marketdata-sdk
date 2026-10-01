package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeDisconnectInfo implements FfiConverterRustBuffer<DisconnectInfo> {
  INSTANCE;

  @Override
  public DisconnectInfo read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeDisconnectInfo.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(DisconnectInfo value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeDisconnectInfo.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(DisconnectInfo value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeDisconnectInfo.INSTANCE.write(value, buf);
    }
  }
}



