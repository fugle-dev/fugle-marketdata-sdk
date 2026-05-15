package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

// public class TestForOptionals {}
public enum FfiConverterOptionalTypeTradingHalt implements FfiConverterRustBuffer<TradingHalt> {
  INSTANCE;

  @Override
  public TradingHalt read(ByteBuffer buf) {
    if (buf.get() == (byte)0) {
      return null;
    }
    return FfiConverterTypeTradingHalt.INSTANCE.read(buf);
  }

  @Override
  public long allocationSize(TradingHalt value) {
    if (value == null) {
      return 1L;
    } else {
      return 1L + FfiConverterTypeTradingHalt.INSTANCE.allocationSize(value);
    }
  }

  @Override
  public void write(TradingHalt value, ByteBuffer buf) {
    if (value == null) {
      buf.put((byte)0);
    } else {
      buf.put((byte)1);
      FfiConverterTypeTradingHalt.INSTANCE.write(value, buf);
    }
  }
}



/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
