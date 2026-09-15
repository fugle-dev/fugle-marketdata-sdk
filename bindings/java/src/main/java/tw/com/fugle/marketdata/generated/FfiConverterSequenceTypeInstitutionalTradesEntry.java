package tw.com.fugle.marketdata.generated;


import java.util.List;
import java.nio.ByteBuffer;
import java.util.stream.IntStream;
import java.util.stream.Stream;

public enum FfiConverterSequenceTypeInstitutionalTradesEntry implements FfiConverterRustBuffer<List<InstitutionalTradesEntry>> {
  INSTANCE;

  @Override
  public List<InstitutionalTradesEntry> read(ByteBuffer buf) {
    int len = buf.getInt();
    return IntStream.range(0, len).mapToObj(_i -> FfiConverterTypeInstitutionalTradesEntry.INSTANCE.read(buf)).toList();
  }

  @Override
  public long allocationSize(List<InstitutionalTradesEntry> value) {
    long sizeForLength = 4L;
    long sizeForItems = value.stream().mapToLong(inner -> FfiConverterTypeInstitutionalTradesEntry.INSTANCE.allocationSize(inner)).sum();
    return sizeForLength + sizeForItems;
  }

  @Override
  public void write(List<InstitutionalTradesEntry> value, ByteBuffer buf) {
    buf.putInt(value.size());
    value.forEach(inner -> FfiConverterTypeInstitutionalTradesEntry.INSTANCE.write(inner, buf));
  }
}



/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
