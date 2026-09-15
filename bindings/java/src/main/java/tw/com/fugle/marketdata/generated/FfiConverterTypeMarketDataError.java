package tw.com.fugle.marketdata.generated;


import java.nio.ByteBuffer;

public enum FfiConverterTypeMarketDataError implements FfiConverterRustBuffer<MarketDataException> {
    INSTANCE;

    @Override
    public MarketDataException read(ByteBuffer buf) {

        return switch(buf.getInt()) {
            case 1 -> new MarketDataException.NetworkException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 2 -> new MarketDataException.AuthException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 3 -> new MarketDataException.RateLimitException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 4 -> new MarketDataException.InvalidSymbol(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 5 -> new MarketDataException.ParseException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 6 -> new MarketDataException.TimeoutException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 7 -> new MarketDataException.WebSocketException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 8 -> new MarketDataException.ClientClosed();
            case 9 -> new MarketDataException.ConfigException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 10 -> new MarketDataException.ApiException(
                FfiConverterString.INSTANCE.read(buf)
                );
            case 11 -> new MarketDataException.Other(
                FfiConverterString.INSTANCE.read(buf)
                );
            default -> throw new RuntimeException("invalid error enum value, something is very wrong!!");
        };
    }

    @Override
    public long allocationSize(MarketDataException value) {
        return switch(value) {
            case MarketDataException.NetworkException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.AuthException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.RateLimitException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.InvalidSymbol x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.ParseException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.TimeoutException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.WebSocketException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.ClientClosed x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
            );
            case MarketDataException.ConfigException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.ApiException x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            case MarketDataException.Other x -> (
                // Add the size for the Int that specifies the variant plus the size needed for all fields
                4L
                + FfiConverterString.INSTANCE.allocationSize(x.msg)
            );
            default -> throw new RuntimeException("invalid error enum value, something is very wrong!!");
        };
    }

    @Override
    public void write(MarketDataException value, ByteBuffer buf) {
        switch(value) {
            case MarketDataException.NetworkException x -> {
                buf.putInt(1);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.AuthException x -> {
                buf.putInt(2);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.RateLimitException x -> {
                buf.putInt(3);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.InvalidSymbol x -> {
                buf.putInt(4);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.ParseException x -> {
                buf.putInt(5);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.TimeoutException x -> {
                buf.putInt(6);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.WebSocketException x -> {
                buf.putInt(7);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.ClientClosed x -> {
                buf.putInt(8);
            }
            case MarketDataException.ConfigException x -> {
                buf.putInt(9);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.ApiException x -> {
                buf.putInt(10);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            case MarketDataException.Other x -> {
                buf.putInt(11);
                FfiConverterString.INSTANCE.write(x.msg, buf);
            }
            default -> throw new RuntimeException("invalid error enum value, something is very wrong!!");
        };
    }
}


