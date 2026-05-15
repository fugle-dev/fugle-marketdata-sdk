package tw.com.fugle.marketdata.generated;


public interface UniffiRustCallStatusErrorHandler<E extends Exception> {
    E lift(RustBuffer.ByValue errorBuf);
}

