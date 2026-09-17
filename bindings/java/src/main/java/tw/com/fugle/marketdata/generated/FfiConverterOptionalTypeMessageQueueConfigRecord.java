package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeMessageQueueConfigRecord implements FfiConverterRustBuffer<MessageQueueConfigRecord> {
  INSTANCE;

  @Override
  public MessageQueueConfigRecord read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeMessageQueueConfigRecord.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(MessageQueueConfigRecord value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeMessageQueueConfigRecord.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(MessageQueueConfigRecord value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeMessageQueueConfigRecord.INSTANCE.write(value, buf);
    }
  }
}



