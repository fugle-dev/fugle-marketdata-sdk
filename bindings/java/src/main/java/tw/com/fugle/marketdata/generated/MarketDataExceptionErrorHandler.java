package tw.com.fugle.marketdata.generated;


public class MarketDataExceptionErrorHandler implements UniffiRustCallStatusErrorHandler<MarketDataException> {
  @Override
  public MarketDataException lift(RustBuffer.ByValue errorBuf){
     return FfiConverterTypeMarketDataError.INSTANCE.lift(errorBuf);
  }
}

