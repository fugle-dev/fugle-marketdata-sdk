package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeListingApplicantsResponse implements FfiConverterRustBuffer<ListingApplicantsResponse> {
  INSTANCE;

  @Override
  public ListingApplicantsResponse read(ByteBuffer buf) {
    return new ListingApplicantsResponse(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterSequenceTypeListingApplicant.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ListingApplicantsResponse value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.dataType()) +
            FfiConverterString.INSTANCE.allocationSize(value.exchange()) +
            FfiConverterString.INSTANCE.allocationSize(value.market()) +
            FfiConverterSequenceTypeListingApplicant.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(ListingApplicantsResponse value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.dataType(), buf);
      FfiConverterString.INSTANCE.write(value.exchange(), buf);
      FfiConverterString.INSTANCE.write(value.market(), buf);
      FfiConverterSequenceTypeListingApplicant.INSTANCE.write(value.data(), buf);
  }
}



