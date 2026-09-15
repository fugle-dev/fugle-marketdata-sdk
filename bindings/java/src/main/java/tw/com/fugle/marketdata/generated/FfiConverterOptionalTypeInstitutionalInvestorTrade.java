package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeInstitutionalInvestorTrade implements FfiConverterRustBuffer<InstitutionalInvestorTrade> {
  INSTANCE;

  @Override
  public InstitutionalInvestorTrade read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeInstitutionalInvestorTrade.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(InstitutionalInvestorTrade value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeInstitutionalInvestorTrade.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(InstitutionalInvestorTrade value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeInstitutionalInvestorTrade.INSTANCE.write(value, buf);
    }
  }
}



