package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeDisconnectInfo implements FfiConverterRustBuffer<DisconnectInfo> {
  INSTANCE;

  @Override
  public DisconnectInfo read(ByteBuffer buf) {
    return new DisconnectInfo(
      FfiConverterOptionalShort.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterTypeDisconnectIntent.INSTANCE.read(buf),
      FfiConverterBoolean.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(DisconnectInfo value) {
      return (
            FfiConverterOptionalShort.INSTANCE.allocationSize(value.code()) +
            FfiConverterString.INSTANCE.allocationSize(value.reason()) +
            FfiConverterTypeDisconnectIntent.INSTANCE.allocationSize(value.intent()) +
            FfiConverterBoolean.INSTANCE.allocationSize(value.willReconnect())
      );
  }

  @Override
  public void write(DisconnectInfo value, ByteBuffer buf) {
      FfiConverterOptionalShort.INSTANCE.write(value.code(), buf);
      FfiConverterString.INSTANCE.write(value.reason(), buf);
      FfiConverterTypeDisconnectIntent.INSTANCE.write(value.intent(), buf);
      FfiConverterBoolean.INSTANCE.write(value.willReconnect(), buf);
  }
}



