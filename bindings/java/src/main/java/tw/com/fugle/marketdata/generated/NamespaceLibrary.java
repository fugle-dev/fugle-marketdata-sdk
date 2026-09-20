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
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_candles_sync() != ((short) 48969)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync() != ((short) 9970)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles() != ((short) 29989)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily() != ((short) 22534)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync() != ((short) 6239)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles() != ((short) 4495)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products() != ((short) 10990)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote() != ((short) 21124)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker() != ((short) 3592)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers() != ((short) 20343)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades() != ((short) 25508)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes() != ((short) 30496)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync() != ((short) 26308)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync() != ((short) 54590)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync() != ((short) 57757)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync() != ((short) 25670)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync() != ((short) 53906)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync() != ((short) 46081)) {
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
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_capital_changes_sync() != ((short) 44530)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync() != ((short) 35826)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes() != ((short) 41161)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends() != ((short) 53857)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants() != ((short) 18770)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync() != ((short) 37063)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync() != ((short) 61155)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles() != ((short) 18890)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats() != ((short) 37563)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync() != ((short) 20776)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync() != ((short) 39759)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles() != ((short) 12448)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote() != ((short) 43288)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker() != ((short) 19948)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers() != ((short) 41778)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades() != ((short) 20755)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes() != ((short) 7709)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync() != ((short) 62355)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync() != ((short) 37699)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync() != ((short) 53677)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync() != ((short) 6270)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync() != ((short) 33858)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_director_holdings_sync() != ((short) 53633)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_etf_holdings_sync() != ((short) 61307)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_director_holdings() != ((short) 46160)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_etf_holdings() != ((short) 51689)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_institutional_trades() != ((short) 22863)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_get_tdcc_distribution() != ((short) 14404)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_institutional_trades_sync() != ((short) 11313)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockownershipclient_tdcc_distribution_sync() != ((short) 57031)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync() != ((short) 40591)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives() != ((short) 29173)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers() != ((short) 51611)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes() != ((short) 51655)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync() != ((short) 41234)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync() != ((short) 31044)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync() != ((short) 23057)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb() != ((short) 542)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj() != ((short) 42166)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd() != ((short) 52544)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi() != ((short) 21456)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma() != ((short) 3997)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync() != ((short) 57078)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync() != ((short) 3744)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync() != ((short) 14395)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync() != ((short) 62329)) {
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
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe() != ((short) 4743)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe() != ((short) 49934)) {
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
