package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeListingApplicant implements FfiConverterRustBuffer<ListingApplicant> {
  INSTANCE;

  @Override
  public ListingApplicant read(ByteBuffer buf) {
    return new ListingApplicant(
      FfiConverterString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ListingApplicant value) {
      return (
            FfiConverterString.INSTANCE.allocationSize(value.symbol()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.name()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.applicationDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.listingDate()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.status()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.industry())
      );
  }

  @Override
  public void write(ListingApplicant value, ByteBuffer buf) {
      FfiConverterString.INSTANCE.write(value.symbol(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.name(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.applicationDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.listingDate(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.status(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.industry(), buf);
  }
}



