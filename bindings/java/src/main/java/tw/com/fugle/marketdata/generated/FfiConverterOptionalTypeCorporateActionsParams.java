package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeCorporateActionsParams implements FfiConverterRustBuffer<CorporateActionsParams> {
  INSTANCE;

  @Override
  public CorporateActionsParams read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeCorporateActionsParams.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(CorporateActionsParams value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeCorporateActionsParams.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(CorporateActionsParams value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeCorporateActionsParams.INSTANCE.write(value, buf);
    }
  }
}



