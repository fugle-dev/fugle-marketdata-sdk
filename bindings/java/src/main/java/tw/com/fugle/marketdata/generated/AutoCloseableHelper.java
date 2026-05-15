package tw.com.fugle.marketdata.generated;


import java.util.stream.Stream;

public interface AutoCloseableHelper {
    static void close(Object... args) {
        Stream.of(args)
              .filter(AutoCloseable.class::isInstance)
              .map(AutoCloseable.class::cast)
              .forEach(closable -> { 
                  try {
                      closable.close();
                  } catch (Exception e) {
                      throw new RuntimeException(e);
                  }
              });
    }
}

