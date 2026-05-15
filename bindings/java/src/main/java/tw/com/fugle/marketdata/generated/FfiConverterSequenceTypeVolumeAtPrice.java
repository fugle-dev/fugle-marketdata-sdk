package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.nio.ByteBuffer;
import java.util.stream.IntStream;
import java.util.stream.Stream;

public enum FfiConverterSequenceTypeVolumeAtPrice implements FfiConverterRustBuffer<List<VolumeAtPrice>> {
  INSTANCE;

  @Override
  public List<VolumeAtPrice> read(ByteBuffer buf) {
    int len = buf.getInt();
    return IntStream.range(0, len).mapToObj(_i -> FfiConverterTypeVolumeAtPrice.INSTANCE.read(buf)).toList();
  }

  @Override
  public long allocationSize(List<VolumeAtPrice> value) {
    long sizeForLength = 4L;
    long sizeForItems = value.stream().mapToLong(inner -> FfiConverterTypeVolumeAtPrice.INSTANCE.allocationSize(inner)).sum();
    return sizeForLength + sizeForItems;
  }

  @Override
  public void write(List<VolumeAtPrice> value, ByteBuffer buf) {
    buf.putInt(value.size());
    value.forEach(inner -> FfiConverterTypeVolumeAtPrice.INSTANCE.write(inner, buf));
  }
}

