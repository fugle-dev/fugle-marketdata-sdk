package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;
import java.util.Map;
import java.util.HashMap;
import java.util.List;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import java.util.stream.Collectors;

public enum FfiConverterMapStringString implements FfiConverterRustBuffer<Map<String, String>> {
    INSTANCE;

    @Override
    public Map<String, String> read(ByteBuffer buf) {
        int len = buf.getInt();
        // Collectors.toMap would be preferred here, but theres a bug that doesn't allow
        // null values in the map, even though that is valid Java
        return IntStream.range(0, len).boxed().collect(
            HashMap::new,
            (m, v) -> m.put(
                FfiConverterString.INSTANCE.read(buf),
                FfiConverterString.INSTANCE.read(buf)
            ),
            HashMap::putAll
        );
    }

    @Override
    public long allocationSize(Map<String, String> value) {
        long spaceForMapSize = 4;
        long spaceForChildren = value.entrySet().stream().mapToLong(entry ->
            FfiConverterString.INSTANCE.allocationSize(entry.getKey()) +
            FfiConverterString.INSTANCE.allocationSize(entry.getValue())
        ).sum();
        return spaceForMapSize + spaceForChildren;
    }

    @Override
    public void write(Map<String, String> value, ByteBuffer buf) {
        buf.putInt(value.size());
        // The parens on `(k, v)` here ensure we're calling the right method,
        // which is important for compatibility with older android devices.
        // Ref https://blog.danlew.net/2017/03/16/kotlin-puzzler-whose-line-is-it-anyways/
        for (var entry : value.entrySet()) {
            FfiConverterString.INSTANCE.write(entry.getKey(), buf);
            FfiConverterString.INSTANCE.write(entry.getValue(), buf);
        }
    }
}

