package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeVolumeAtPrice implements FfiConverterRustBuffer<VolumeAtPrice> {
  INSTANCE;

  @Override
  public VolumeAtPrice read(ByteBuffer buf) {
    return new VolumeAtPrice(
      FfiConverterDouble.INSTANCE.read(buf),
      FfiConverterLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf),
      FfiConverterOptionalLong.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(VolumeAtPrice value) {
      return (
            FfiConverterDouble.INSTANCE.allocationSize(value.price()) +
            FfiConverterLong.INSTANCE.allocationSize(value.volume()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.volumeAtBid()) +
            FfiConverterOptionalLong.INSTANCE.allocationSize(value.volumeAtAsk())
      );
  }

  @Override
  public void write(VolumeAtPrice value, ByteBuffer buf) {
      FfiConverterDouble.INSTANCE.write(value.price(), buf);
      FfiConverterLong.INSTANCE.write(value.volume(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.volumeAtBid(), buf);
      FfiConverterOptionalLong.INSTANCE.write(value.volumeAtAsk(), buf);
  }
}



