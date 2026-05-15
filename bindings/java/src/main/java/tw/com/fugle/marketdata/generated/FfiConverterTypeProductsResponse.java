package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeProductsResponse implements FfiConverterRustBuffer<ProductsResponse> {
  INSTANCE;

  @Override
  public ProductsResponse read(ByteBuffer buf) {
    return new ProductsResponse(
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterOptionalString.INSTANCE.read(buf),
      FfiConverterSequenceTypeProduct.INSTANCE.read(buf)
    );
  }

  @Override
  public long allocationSize(ProductsResponse value) {
      return (
            FfiConverterOptionalString.INSTANCE.allocationSize(value.date()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.productType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.session()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.contractType()) +
            FfiConverterOptionalString.INSTANCE.allocationSize(value.status()) +
            FfiConverterSequenceTypeProduct.INSTANCE.allocationSize(value.data())
      );
  }

  @Override
  public void write(ProductsResponse value, ByteBuffer buf) {
      FfiConverterOptionalString.INSTANCE.write(value.date(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.productType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.session(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.contractType(), buf);
      FfiConverterOptionalString.INSTANCE.write(value.status(), buf);
      FfiConverterSequenceTypeProduct.INSTANCE.write(value.data(), buf);
  }
}



