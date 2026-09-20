package tw.com.fugle.marketdata.generated;


import com.sun.jna.Library;
import com.sun.jna.Native;

final class NamespaceLibrary {
  static synchronized String findLibraryName(String componentName) {
    String libOverride = System.getProperty("uniffi.component." + componentName + ".libraryOverride");
    if (libOverride != null) {
        return libOverride;
    }
    return "marketdata_uniffi";
  }

  static <Lib extends Library> Lib loadIndirect(String componentName, Class<Lib> clazz) {
    return Native.load(findLibraryName(componentName), clazz);
  }

  static void uniffiCheckContractApiVersion(UniffiLib lib) {
    // Get the bindings contract version from our ComponentInterface
    int bindingsContractVersion = 29;
    // Get the scaffolding contract version by calling the into the dylib
    int scaffoldingContractVersion = lib.ffi_marketdata_uniffi_uniffi_contract_version();
    if (bindingsContractVersion != scaffoldingContractVersion) {
        throw new RuntimeException("UniFFI contract version mismatch: try cleaning and rebuilding your project");
    }
  }

  static void uniffiCheckApiChecksums(UniffiLib lib) {
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key() != ((short) 2560)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key_and_tls() != ((short) 17616)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token() != ((short) 30582)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token_and_tls() != ((short) 21309)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token() != ((short) 14209)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token_and_tls() != ((short) 25673)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_websocket_client() != ((short) 17568)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_config() != ((short) 19180)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_endpoint() != ((short) 15148)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_func_validate_credentials() != ((short) 23718)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptclient_historical() != ((short) 18194)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptclient_intraday() != ((short) 43120)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_candles_sync() != ((short) 8321)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync() != ((short) 43568)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles() != ((short) 6749)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily() != ((short) 61915)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync() != ((short) 15435)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles() != ((short) 18846)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products() != ((short) 28718)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote() != ((short) 60925)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker() != ((short) 6755)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers() != ((short) 2210)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades() != ((short) 18238)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes() != ((short) 46935)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync() != ((short) 21712)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync() != ((short) 49258)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync() != ((short) 25885)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync() != ((short) 32515)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync() != ((short) 4933)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync() != ((short) 40116)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_restclient_base_url() != ((short) 36384)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_restclient_futopt() != ((short) 65348)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_restclient_stock() != ((short) 18733)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_base_url() != ((short) 28231)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_corporate_actions() != ((short) 38783)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_historical() != ((short) 45578)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_intraday() != ((short) 53228)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_ownership() != ((short) 26642)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_snapshot() != ((short) 49856)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_technical() != ((short) 10974)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_capital_changes_sync() != ((short) 31988)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync() != ((short) 37175)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes() != ((short) 22794)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends() != ((short) 1657)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants() != ((short) 1735)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync() != ((short) 4098)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync() != ((short) 16718)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles() != ((short) 30527)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats() != ((short) 37563)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync() != ((short) 20776)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync() != ((short) 43276)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles() != ((short) 27303)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote() != ((short) 4800)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker() != ((short) 11469)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers() != ((short) 25432)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades() != ((short) 49659)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes() != ((short) 7081)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync() != ((short) 14450)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync() != ((short) 12061)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync() != ((short) 50959)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync() != ((short) 17544)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync() != ((short) 53696)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_director_holdings_sync() != ((short) 42397)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_etf_holdings_sync() != ((short) 9047)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_director_holdings() != ((short) 9436)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_etf_holdings() != ((short) 32666)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_institutional_trades() != ((short) 44140)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_tdcc_distribution() != ((short) 55570)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_institutional_trades_sync() != ((short) 27652)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_tdcc_distribution_sync() != ((short) 41522)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync() != ((short) 34682)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives() != ((short) 54146)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers() != ((short) 19121)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes() != ((short) 18220)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync() != ((short) 58532)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync() != ((short) 8259)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync() != ((short) 60077)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb() != ((short) 16142)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj() != ((short) 15872)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd() != ((short) 61798)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi() != ((short) 3410)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma() != ((short) 28284)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync() != ((short) 7023)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync() != ((short) 3187)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync() != ((short) 43008)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync() != ((short) 21533)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_connect() != ((short) 34522)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect() != ((short) 58180)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed() != ((short) 1028)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected() != ((short) 18665)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_measure_latency() != ((short) 53522)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_messages_dropped_total() != ((short) 28793)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_ping() != ((short) 51664)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_query_subscriptions() != ((short) 20069)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe() != ((short) 12456)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe() != ((short) 6177)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe_ids() != ((short) 5738)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected() != ((short) 42437)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_authenticated() != ((short) 51034)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_unauthenticated() != ((short) 29216)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected() != ((short) 44379)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message() != ((short) 4936)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error() != ((short) 44329)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting() != ((short) 12322)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed() != ((short) 46093)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_messages_dropped() != ((short) 34523)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new() != ((short) 36225)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_config() != ((short) 8956)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_credentials() != ((short) 53902)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_endpoint() != ((short) 35702)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_full_config() != ((short) 32798)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_options() != ((short) 2558)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_url() != ((short) 63549)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
  }
}

// Define FFI callback types
