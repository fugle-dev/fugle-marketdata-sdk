package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;

import java.util.concurrent.CompletableFuture;

/**
 * Main entry point for the Fugle MarketData SDK.
 *
 * Use the builder pattern to create a client with your preferred authentication method:
 * <pre>{@code
 * FugleRestClient client = FugleRestClient.builder()
 *     .apiKey("YOUR_API_KEY")
 *     .build();
 *
 * // Sync method - blocks and returns result
 * Quote quote = client.stock().intraday().getQuote("2330");
 *
 * // Async method - returns CompletableFuture
 * CompletableFuture<Quote> future = client.stock().intraday().getQuoteAsync("2330");
 * }</pre>
 */
public class FugleRestClient implements AutoCloseable {

    private final RestClient restClient;
    private final StockClientWrapper stockClient;
    private final FutOptClientWrapper futOptClient;

    private FugleRestClient(RestClient restClient) {
        this.restClient = restClient;
        this.stockClient = new StockClientWrapper(restClient.stock());
        this.futOptClient = new FutOptClientWrapper(restClient.futopt());
    }

    /**
     * Get the stock market data client.
     */
    public StockClientWrapper stock() {
        return stockClient;
    }

    /**
     * Get the futures and options market data client.
     */
    public FutOptClientWrapper futopt() {
        return futOptClient;
    }

    /**
     * Get the futures and options market data client (alias for futopt()).
     */
    public FutOptClientWrapper futOpt() {
        return futOptClient;
    }

    @Override
    public void close() {
        restClient.close();
    }

    /**
     * Create a new builder for constructing a FugleRestClient.
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for creating FugleRestClient instances.
     *
     * Supports three authentication methods:
     * - API Key (most common)
     * - Bearer Token (OAuth)
     * - SDK Token (legacy)
     */
    public static class Builder {
        private String apiKey;
        private String bearerToken;
        private String sdkToken;
        private String baseUrl;

        private Builder() {}

        /**
         * Set API key for authentication.
         */
        public Builder apiKey(String apiKey) {
            this.apiKey = apiKey;
            return this;
        }

        /**
         * Set bearer token for OAuth authentication.
         */
        public Builder bearerToken(String bearerToken) {
            this.bearerToken = bearerToken;
            return this;
        }

        /**
         * Set SDK token for legacy authentication.
         */
        public Builder sdkToken(String sdkToken) {
            this.sdkToken = sdkToken;
            return this;
        }

        /**
         * Set custom base URL for API endpoint.
         *
         * @param baseUrl Custom base URL (e.g., "https://custom.api.fugle.tw")
         * @return This builder for chaining
         */
        public Builder baseUrl(String baseUrl) {
            this.baseUrl = baseUrl;
            return this;
        }

        /**
         * Build the FugleRestClient.
         *
         * @throws FugleException if exactly one authentication method is not provided or if client creation fails
         */
        public FugleRestClient build() {
            try {
                // Exactly-one-auth validation
                int authCount = 0;
                if (apiKey != null) authCount++;
                if (bearerToken != null) authCount++;
                if (sdkToken != null) authCount++;

                if (authCount == 0) {
                    throw new FugleException("Provide exactly one of: apiKey, bearerToken, sdkToken");
                }
                if (authCount > 1) {
                    throw new FugleException("Provide exactly one of: apiKey, bearerToken, sdkToken");
                }

                // Create client with appropriate auth method
                RestClient restClient;
                if (apiKey != null) {
                    restClient = MarketdataUniffi.newRestClientWithApiKey(apiKey);
                } else if (bearerToken != null) {
                    restClient = MarketdataUniffi.newRestClientWithBearerToken(bearerToken);
                } else {
                    restClient = MarketdataUniffi.newRestClientWithSdkToken(sdkToken);
                }

                // TODO: baseUrl cannot be set post-construction via UniFFI
                // Core RestClient has base_url() builder method that consumes self.
                // Since UniFFI RestClient wraps Arc, we need a UniFFI-exposed setter.
                // For now, baseUrl is stored but not applied (matches Python/Node.js phases 12-02, 13-02).
                if (baseUrl != null) {
                    // baseUrl stored but not yet applied - requires UniFFI API extension
                }

                return new FugleRestClient(restClient);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }
    }

    /**
     * Wrapper for stock market data client providing idiomatic Java API.
     */
    public static class StockClientWrapper {
        private final StockClient stockClient;
        private final StockIntradayClientWrapper intradayClient;

        private StockClientWrapper(StockClient stockClient) {
            this.stockClient = stockClient;
            this.intradayClient = new StockIntradayClientWrapper(stockClient.intraday());
        }

        /**
         * Get the intraday (real-time) data client.
         */
        public StockIntradayClientWrapper intraday() {
            return intradayClient;
        }

        /**
         * Get the historical data client.
         */
        public StockHistoricalClient historical() {
            return stockClient.historical();
        }

        /**
         * Get the snapshot (market-wide) data client.
         */
        public StockSnapshotClient snapshot() {
            return stockClient.snapshot();
        }

        /**
         * Get the technical indicators client.
         */
        public StockTechnicalClient technical() {
            return stockClient.technical();
        }

        /**
         * Get the corporate actions client.
         */
        public StockCorporateActionsClient corporateActions() {
            return stockClient.corporateActions();
        }
    }

    /**
     * Wrapper for futures/options market data client providing idiomatic Java API.
     */
    public static class FutOptClientWrapper {
        private final FutOptClient futOptClient;
        private final FutOptIntradayClientWrapper intradayClient;

        private FutOptClientWrapper(FutOptClient futOptClient) {
            this.futOptClient = futOptClient;
            this.intradayClient = new FutOptIntradayClientWrapper(futOptClient.intraday());
        }

        /**
         * Get the intraday (real-time) data client.
         */
        public FutOptIntradayClientWrapper intraday() {
            return intradayClient;
        }

        /**
         * Get the historical data client.
         */
        public FutOptHistoricalClient historical() {
            return futOptClient.historical();
        }
    }

    /**
     * Wrapper for stock intraday client with dual sync/async methods.
     */
    public static class StockIntradayClientWrapper {
        private final StockIntradayClient client;

        private StockIntradayClientWrapper(StockIntradayClient client) {
            this.client = client;
        }

        // Async methods (idiomatic Java pattern: getXxx returns CompletableFuture)

        /**
         * Get quote for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing Quote
         */
        public CompletableFuture<Quote> getQuoteAsync(String symbol) {
            return client.getQuote(symbol)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get ticker info for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing Ticker
         */
        public CompletableFuture<Ticker> getTickerAsync(String symbol) {
            return client.getTicker(symbol)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get trade history for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing TradesResponse
         */
        public CompletableFuture<TradesResponse> getTradesAsync(String symbol) {
            return client.getTrades(symbol)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get volume breakdown for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing VolumesResponse
         */
        public CompletableFuture<VolumesResponse> getVolumesAsync(String symbol) {
            return client.getVolumes(symbol)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get candlestick data for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param timeframe Candlestick timeframe: "1", "5", "10", "15", "30", "60" (minutes)
         * @return CompletableFuture containing IntradayCandlesResponse
         */
        public CompletableFuture<IntradayCandlesResponse> getCandlesAsync(String symbol, String timeframe) {
            return client.getCandles(symbol, timeframe)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        // Sync methods (block and return result directly)

        /**
         * Get quote for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return Quote
         * @throws FugleException if the request fails
         */
        public Quote getQuote(String symbol) {
            try {
                return client.quoteSync(symbol);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get ticker info for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return Ticker
         * @throws FugleException if the request fails
         */
        public Ticker getTicker(String symbol) {
            try {
                return client.tickerSync(symbol);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get trade history for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return TradesResponse
         * @throws FugleException if the request fails
         */
        public TradesResponse getTrades(String symbol) {
            try {
                return client.tradesSync(symbol);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get volume breakdown for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return VolumesResponse
         * @throws FugleException if the request fails
         */
        public VolumesResponse getVolumes(String symbol) {
            try {
                return client.volumesSync(symbol);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get candlestick data for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param timeframe Candlestick timeframe: "1", "5", "10", "15", "30", "60" (minutes)
         * @return IntradayCandlesResponse
         * @throws FugleException if the request fails
         */
        public IntradayCandlesResponse getCandles(String symbol, String timeframe) {
            try {
                return client.candlesSync(symbol, timeframe);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }
    }

    /**
     * Wrapper for futures/options intraday client with dual sync/async methods.
     */
    public static class FutOptIntradayClientWrapper {
        private final FutOptIntradayClient client;

        private FutOptIntradayClientWrapper(FutOptIntradayClient client) {
            this.client = client;
        }

        // Async methods

        /**
         * Get quote for a futures/options contract (async).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param afterHours true for after-hours session
         * @return CompletableFuture containing FutOptQuote
         */
        public CompletableFuture<FutOptQuote> getQuoteAsync(String symbol, Boolean afterHours) {
            return client.getQuote(symbol, afterHours)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get quote for a futures/options contract (async, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return CompletableFuture containing FutOptQuote
         */
        public CompletableFuture<FutOptQuote> getQuoteAsync(String symbol) {
            return getQuoteAsync(symbol, false);
        }

        /**
         * Get ticker info for a futures/options contract (async).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param afterHours true for after-hours session
         * @return CompletableFuture containing FutOptTicker
         */
        public CompletableFuture<FutOptTicker> getTickerAsync(String symbol, Boolean afterHours) {
            return client.getTicker(symbol, afterHours)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get ticker info for a futures/options contract (async, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return CompletableFuture containing FutOptTicker
         */
        public CompletableFuture<FutOptTicker> getTickerAsync(String symbol) {
            return getTickerAsync(symbol, false);
        }

        /**
         * Get available products list (async).
         *
         * @param type "F" for futures, "O" for options
         * @return CompletableFuture containing ProductsResponse
         */
        public CompletableFuture<ProductsResponse> getProductsAsync(String type) {
            return client.getProducts(type)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        // Sync methods

        /**
         * Get quote for a futures/options contract (sync/blocking).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param afterHours true for after-hours session
         * @return FutOptQuote
         * @throws FugleException if the request fails
         */
        public FutOptQuote getQuote(String symbol, Boolean afterHours) {
            try {
                return client.quoteSync(symbol, afterHours);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get quote for a futures/options contract (sync/blocking, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return FutOptQuote
         * @throws FugleException if the request fails
         */
        public FutOptQuote getQuote(String symbol) {
            return getQuote(symbol, false);
        }

        /**
         * Get ticker info for a futures/options contract (sync/blocking).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param afterHours true for after-hours session
         * @return FutOptTicker
         * @throws FugleException if the request fails
         */
        public FutOptTicker getTicker(String symbol, Boolean afterHours) {
            try {
                return client.tickerSync(symbol, afterHours);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get ticker info for a futures/options contract (sync/blocking, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return FutOptTicker
         * @throws FugleException if the request fails
         */
        public FutOptTicker getTicker(String symbol) {
            return getTicker(symbol, false);
        }

        /**
         * Get available products list (sync/blocking).
         *
         * @param type "F" for futures, "O" for options
         * @return ProductsResponse
         * @throws FugleException if the request fails
         */
        public ProductsResponse getProducts(String type) {
            try {
                return client.productsSync(type);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }
    }
}
