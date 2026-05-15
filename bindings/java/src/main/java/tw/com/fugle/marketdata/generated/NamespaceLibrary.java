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
    int bindingsContractVersion = 26;
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
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptclient_historical() != ((short) 18194)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptclient_intraday() != ((short) 43120)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_candles_sync() != ((short) 56503)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_daily_sync() != ((short) 10493)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_candles() != ((short) 14488)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futopthistoricalclient_get_daily() != ((short) 41351)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_candles_sync() != ((short) 60136)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_candles() != ((short) 62970)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_products() != ((short) 61510)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_quote() != ((short) 21333)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_ticker() != ((short) 30953)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_tickers() != ((short) 29913)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_trades() != ((short) 3561)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_get_volumes() != ((short) 5062)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_products_sync() != ((short) 8976)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_quote_sync() != ((short) 33593)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_ticker_sync() != ((short) 53319)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_tickers_sync() != ((short) 55649)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_trades_sync() != ((short) 987)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_futoptintradayclient_volumes_sync() != ((short) 25987)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_restclient_futopt() != ((short) 65348)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_restclient_stock() != ((short) 18733)) {
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
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_snapshot() != ((short) 49856)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockclient_technical() != ((short) 10974)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_capital_changes_sync() != ((short) 38225)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_dividends_sync() != ((short) 26469)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_capital_changes() != ((short) 34953)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_dividends() != ((short) 30186)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_get_listing_applicants() != ((short) 41091)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockcorporateactionsclient_listing_applicants_sync() != ((short) 19487)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_candles_sync() != ((short) 58660)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_candles() != ((short) 18842)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_get_stats() != ((short) 19930)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockhistoricalclient_stats_sync() != ((short) 25283)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_candles_sync() != ((short) 10535)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_candles() != ((short) 20034)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_quote() != ((short) 64785)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_ticker() != ((short) 26620)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_tickers() != ((short) 24176)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_trades() != ((short) 48306)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_get_volumes() != ((short) 41478)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_quote_sync() != ((short) 24390)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_ticker_sync() != ((short) 22635)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_tickers_sync() != ((short) 3916)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_trades_sync() != ((short) 4040)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stockintradayclient_volumes_sync() != ((short) 8850)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_actives_sync() != ((short) 45448)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_actives() != ((short) 31681)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_movers() != ((short) 54795)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_get_quotes() != ((short) 5150)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_movers_sync() != ((short) 38625)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocksnapshotclient_quotes_sync() != ((short) 7562)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_bb_sync() != ((short) 50012)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_bb() != ((short) 28523)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_kdj() != ((short) 47666)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_macd() != ((short) 17293)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_rsi() != ((short) 23780)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_get_sma() != ((short) 37856)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_kdj_sync() != ((short) 58302)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_macd_sync() != ((short) 32247)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_rsi_sync() != ((short) 6527)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_stocktechnicalclient_sma_sync() != ((short) 18246)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_connect() != ((short) 34522)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect() != ((short) 57258)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed() != ((short) 15116)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected() != ((short) 53625)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_ping() != ((short) 51664)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_query_subscriptions() != ((short) 20069)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe() != ((short) 39559)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe() != ((short) 21735)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected() != ((short) 56842)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected() != ((short) 54477)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message() != ((short) 54327)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error() != ((short) 64085)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting() != ((short) 63646)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed() != ((short) 36342)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new() != ((short) 36225)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_config() != ((short) 8956)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_endpoint() != ((short) 35702)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_full_config() != ((short) 39727)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
    if (lib.uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_url() != ((short) 63549)) {
        throw new RuntimeException("UniFFI API checksum mismatch: try cleaning and rebuilding your project");
    }
  }
}

// Define FFI callback types
