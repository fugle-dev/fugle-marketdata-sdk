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
 * // REST methods return the server's JSON response verbatim; decode it
 * // with whatever JSON library you already use.
 * String json = client.stock().intraday().getQuote("2330");
 *
 * // Async method - returns CompletableFuture
 * CompletableFuture<String> future = client.stock().intraday().getQuoteAsync("2330");
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
         * @throws FugleException with code 1004 if not exactly one non-empty
         *     authentication method is provided (empty or whitespace-only values
         *     count as not provided), or if client creation fails
         */
        public FugleRestClient build() {
            try {
                // Core requires exactly one non-blank credential (ConfigError,
                // code 1004) and reports which one to use.
                CredentialKind kind = MarketdataUniffi.validateCredentials(apiKey, bearerToken, sdkToken);

                // Create client with appropriate auth method
                RestClient restClient;
                switch (kind) {
                    case API_KEY:
                        restClient = MarketdataUniffi.newRestClientWithApiKey(apiKey);
                        break;
                    case BEARER_TOKEN:
                        restClient = MarketdataUniffi.newRestClientWithBearerToken(bearerToken);
                        break;
                    default:
                        restClient = MarketdataUniffi.newRestClientWithSdkToken(sdkToken);
                        break;
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
        private final StockOwnershipClientWrapper ownershipClient;

        private StockClientWrapper(StockClient stockClient) {
            this.stockClient = stockClient;
            this.intradayClient = new StockIntradayClientWrapper(stockClient.intraday());
            this.ownershipClient = new StockOwnershipClientWrapper(stockClient.ownership());
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

        /**
         * Get the ownership (ETF holdings, institutional trades, director holdings, TDCC distribution) client.
         */
        public StockOwnershipClientWrapper ownership() {
            return ownershipClient;
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
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getQuoteAsync(String symbol) {
            return getQuoteAsync(symbol, null);
        }

        /**
         * Get quote for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot filter), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getQuoteAsync(String symbol, OddLotParams params) {
            return client.getQuote(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get ticker info for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTickerAsync(String symbol) {
            return getTickerAsync(symbol, null);
        }

        /**
         * Get ticker info for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot filter), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTickerAsync(String symbol, OddLotParams params) {
            return client.getTicker(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get trade history for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTradesAsync(String symbol) {
            return getTradesAsync(symbol, null);
        }

        /**
         * Get trade history for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot, offset, limit, sort, isTrial), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTradesAsync(String symbol, StockTradesParams params) {
            return client.getTrades(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get volume breakdown for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getVolumesAsync(String symbol) {
            return getVolumesAsync(symbol, null);
        }

        /**
         * Get volume breakdown for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot filter), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getVolumesAsync(String symbol, OddLotParams params) {
            return client.getVolumes(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get candlestick data for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getCandlesAsync(String symbol) {
            return getCandlesAsync(symbol, null);
        }

        /**
         * Get candlestick data for a symbol (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (timeframe: "1", "5", "10", "15", "30", "60" minutes;
         *     odd-lot filter; sort), or null for the server default (1-minute candles)
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getCandlesAsync(String symbol, StockCandlesParams params) {
            return client.getCandles(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        // Sync methods (block and return result directly)

        /**
         * Get quote for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getQuote(String symbol) {
            return getQuote(symbol, null);
        }

        /**
         * Get quote for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot filter), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getQuote(String symbol, OddLotParams params) {
            try {
                return client.quoteSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get ticker info for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTicker(String symbol) {
            return getTicker(symbol, null);
        }

        /**
         * Get ticker info for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot filter), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTicker(String symbol, OddLotParams params) {
            try {
                return client.tickerSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get trade history for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTrades(String symbol) {
            return getTrades(symbol, null);
        }

        /**
         * Get trade history for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot, offset, limit, sort, isTrial), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTrades(String symbol, StockTradesParams params) {
            try {
                return client.tradesSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get volume breakdown for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getVolumes(String symbol) {
            return getVolumes(symbol, null);
        }

        /**
         * Get volume breakdown for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (odd-lot filter), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getVolumes(String symbol, OddLotParams params) {
            try {
                return client.volumesSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get candlestick data for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getCandles(String symbol) {
            return getCandles(symbol, null);
        }

        /**
         * Get candlestick data for a symbol (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (timeframe: "1", "5", "10", "15", "30", "60" minutes;
         *     odd-lot filter; sort), or null for the server default (1-minute candles)
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getCandles(String symbol, StockCandlesParams params) {
            try {
                return client.candlesSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }
    }

    /**
     * Wrapper for stock ownership client with dual sync/async methods.
     */
    public static class StockOwnershipClientWrapper {
        private final StockOwnershipClient client;

        private StockOwnershipClientWrapper(StockOwnershipClient client) {
            this.client = client;
        }

        // Async methods

        /**
         * Get the constituents an ETF held over a date range (async).
         *
         * @param symbol ETF symbol (e.g., "0050")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getEtfHoldingsAsync(String symbol) {
            return getEtfHoldingsAsync(symbol, null);
        }

        /**
         * Get the constituents an ETF held over a date range (async).
         *
         * @param symbol ETF symbol (e.g., "0050")
         * @param params Query parameters (from, to, sort), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getEtfHoldingsAsync(String symbol, OwnershipParams params) {
            return client.getEtfHoldings(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getInstitutionalTradesAsync(String symbol) {
            return getInstitutionalTradesAsync(symbol, null);
        }

        /**
         * Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (from, to, sort), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getInstitutionalTradesAsync(String symbol, OwnershipParams params) {
            return client.getInstitutionalTrades(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get monthly holdings and pledges disclosed by directors and supervisors (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getDirectorHoldingsAsync(String symbol) {
            return getDirectorHoldingsAsync(symbol, null);
        }

        /**
         * Get monthly holdings and pledges disclosed by directors and supervisors (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (from, to, sort), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getDirectorHoldingsAsync(String symbol, OwnershipParams params) {
            return client.getDirectorHoldings(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get the weekly TDCC shareholder distribution by holding-size bracket (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTdccDistributionAsync(String symbol) {
            return getTdccDistributionAsync(symbol, null);
        }

        /**
         * Get the weekly TDCC shareholder distribution by holding-size bracket (async).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (from, to, sort), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTdccDistributionAsync(String symbol, OwnershipParams params) {
            return client.getTdccDistribution(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        // Sync methods

        /**
         * Get the constituents an ETF held over a date range (sync/blocking).
         *
         * @param symbol ETF symbol (e.g., "0050")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getEtfHoldings(String symbol) {
            return getEtfHoldings(symbol, null);
        }

        /**
         * Get the constituents an ETF held over a date range (sync/blocking).
         *
         * @param symbol ETF symbol (e.g., "0050")
         * @param params Query parameters (from, to, sort), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getEtfHoldings(String symbol, OwnershipParams params) {
            try {
                return client.etfHoldingsSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getInstitutionalTrades(String symbol) {
            return getInstitutionalTrades(symbol, null);
        }

        /**
         * Get daily trading by the three major institutional investors (foreign, investment trust, dealer) (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (from, to, sort), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getInstitutionalTrades(String symbol, OwnershipParams params) {
            try {
                return client.institutionalTradesSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getDirectorHoldings(String symbol) {
            return getDirectorHoldings(symbol, null);
        }

        /**
         * Get monthly holdings and pledges disclosed by directors and supervisors (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (from, to, sort), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getDirectorHoldings(String symbol, OwnershipParams params) {
            try {
                return client.directorHoldingsSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTdccDistribution(String symbol) {
            return getTdccDistribution(symbol, null);
        }

        /**
         * Get the weekly TDCC shareholder distribution by holding-size bracket (sync/blocking).
         *
         * @param symbol Stock symbol (e.g., "2330")
         * @param params Query parameters (from, to, sort), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTdccDistribution(String symbol, OwnershipParams params) {
            try {
                return client.tdccDistributionSync(symbol, params);
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
         * Get quote for a futures/options contract (async, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getQuoteAsync(String symbol) {
            return getQuoteAsync(symbol, null);
        }

        /**
         * Get quote for a futures/options contract (async).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param params Query parameters (after-hours session), or null for regular hours
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getQuoteAsync(String symbol, AfterHoursParams params) {
            return client.getQuote(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get ticker info for a futures/options contract (async, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTickerAsync(String symbol) {
            return getTickerAsync(symbol, null);
        }

        /**
         * Get ticker info for a futures/options contract (async).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param params Query parameters (after-hours session), or null for regular hours
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getTickerAsync(String symbol, AfterHoursParams params) {
            return client.getTicker(symbol, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        /**
         * Get available products list (async).
         *
         * @param type "F" for futures, "O" for options
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getProductsAsync(String type) {
            return getProductsAsync(type, null);
        }

        /**
         * Get available products list (async).
         *
         * @param type "F" for futures, "O" for options
         * @param params Query parameters (exchange, after-hours session, contract type, status), or null for none
         * @return CompletableFuture containing the response body as JSON
         */
        public CompletableFuture<String> getProductsAsync(String type, FutOptProductsParams params) {
            return client.getProducts(type, params)
                .exceptionally(e -> { throw FugleException.unwrap(e); });
        }

        // Sync methods

        /**
         * Get quote for a futures/options contract (sync/blocking, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getQuote(String symbol) {
            return getQuote(symbol, null);
        }

        /**
         * Get quote for a futures/options contract (sync/blocking).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param params Query parameters (after-hours session), or null for regular hours
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getQuote(String symbol, AfterHoursParams params) {
            try {
                return client.quoteSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get ticker info for a futures/options contract (sync/blocking, regular hours).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTicker(String symbol) {
            return getTicker(symbol, null);
        }

        /**
         * Get ticker info for a futures/options contract (sync/blocking).
         *
         * @param symbol Contract symbol (e.g., "TXFA4")
         * @param params Query parameters (after-hours session), or null for regular hours
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getTicker(String symbol, AfterHoursParams params) {
            try {
                return client.tickerSync(symbol, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }

        /**
         * Get available products list (sync/blocking).
         *
         * @param type "F" for futures, "O" for options
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getProducts(String type) {
            return getProducts(type, null);
        }

        /**
         * Get available products list (sync/blocking).
         *
         * @param type "F" for futures, "O" for options
         * @param params Query parameters (exchange, after-hours session, contract type, status), or null for none
         * @return the response body as JSON
         * @throws FugleException if the request fails
         */
        public String getProducts(String type, FutOptProductsParams params) {
            try {
                return client.productsSync(type, params);
            } catch (MarketDataException e) {
                throw FugleException.from(e);
            }
        }
    }
}
