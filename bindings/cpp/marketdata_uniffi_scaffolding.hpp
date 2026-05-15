#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef UNIFFI_CPP_INTERNALSTRUCTS
#define UNIFFI_CPP_INTERNALSTRUCTS
struct ForeignBytes {
    int32_t len;
    uint8_t *data;
};

struct RustBuffer {
    uint64_t capacity;
    uint64_t len;
    uint8_t *data;
};

struct RustCallStatus {
    int8_t code;
    RustBuffer error_buf;
};

#endif
struct UniffiVTableCallbackInterfaceWebSocketListener {
    void * on_connected;
    void * on_disconnected;
    void * on_message;
    void * on_error;
    void * on_reconnecting;
    void * on_reconnect_failed;
    void * uniffi_free;
};
void * uniffi_marketdata_uniffi_fn_clone_futoptclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_futoptclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_futoptclient_historical(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_futoptclient_intraday(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_futopthistoricalclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_futopthistoricalclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_futoptintradayclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_futoptintradayclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_restclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_restclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_restclient_futopt(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_restclient_stock(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_stockclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_stockclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_stockclient_corporate_actions(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_stockclient_historical(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_stockclient_intraday(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_stockclient_snapshot(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_method_stockclient_technical(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_stockcorporateactionsclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_stockcorporateactionsclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_stockhistoricalclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_stockhistoricalclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_stockintradayclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_stockintradayclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_stocksnapshotclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_stocksnapshotclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_stocktechnicalclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_stocktechnicalclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_websocketclient(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_websocketclient(void * ptr, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_constructor_websocketclient_new(RustBuffer api_key, void * listener, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_config(RustBuffer api_key, void * listener, RustBuffer endpoint, RustBuffer reconnect_config, RustBuffer health_check_config, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_endpoint(RustBuffer api_key, void * listener, RustBuffer endpoint, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_full_config(RustBuffer api_key, void * listener, RustBuffer endpoint, RustBuffer base_url, RustBuffer reconnect_config, RustBuffer health_check_config, RustBuffer tls, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_constructor_websocketclient_new_with_url(RustBuffer api_key, void * listener, RustBuffer endpoint, RustBuffer base_url, RustBuffer reconnect_config, RustBuffer health_check_config, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketclient_connect_sync(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketclient_disconnect_sync(void * ptr, RustCallStatus *out_status);
int8_t uniffi_marketdata_uniffi_fn_method_websocketclient_is_closed(void * ptr, RustCallStatus *out_status);
int8_t uniffi_marketdata_uniffi_fn_method_websocketclient_is_connected(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketclient_ping_sync(void * ptr, RustBuffer state, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketclient_query_subscriptions_sync(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketclient_subscribe_sync(void * ptr, RustBuffer channel, RustBuffer symbol, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketclient_unsubscribe_sync(void * ptr, RustBuffer channel, RustBuffer symbol, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_clone_websocketlistener(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_free_websocketlistener(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_init_callback_vtable_websocketlistener(UniffiVTableCallbackInterfaceWebSocketListener & vtable);
void uniffi_marketdata_uniffi_fn_method_websocketlistener_on_connected(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketlistener_on_disconnected(void * ptr, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketlistener_on_message(void * ptr, RustBuffer message, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketlistener_on_error(void * ptr, RustBuffer error_message, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnecting(void * ptr, uint32_t attempt, RustCallStatus *out_status);
void uniffi_marketdata_uniffi_fn_method_websocketlistener_on_reconnect_failed(void * ptr, uint32_t attempts, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key(RustBuffer api_key, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_rest_client_with_api_key_and_tls(RustBuffer api_key, RustBuffer base_url, RustBuffer tls, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token(RustBuffer bearer_token, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_rest_client_with_bearer_token_and_tls(RustBuffer bearer_token, RustBuffer base_url, RustBuffer tls, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token(RustBuffer sdk_token, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_rest_client_with_sdk_token_and_tls(RustBuffer sdk_token, RustBuffer base_url, RustBuffer tls, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_websocket_client(RustBuffer api_key, void * listener, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_websocket_client_with_config(RustBuffer api_key, void * listener, RustBuffer endpoint, RustBuffer reconnect_config, RustBuffer health_check_config, RustCallStatus *out_status);
void * uniffi_marketdata_uniffi_fn_func_new_websocket_client_with_endpoint(RustBuffer api_key, void * listener, RustBuffer endpoint, RustCallStatus *out_status);
RustBuffer ffi_marketdata_uniffi_rustbuffer_alloc(uint64_t size, RustCallStatus *out_status);
RustBuffer ffi_marketdata_uniffi_rustbuffer_from_bytes(ForeignBytes bytes, RustCallStatus *out_status);
void ffi_marketdata_uniffi_rustbuffer_free(RustBuffer buf, RustCallStatus *out_status);
RustBuffer ffi_marketdata_uniffi_rustbuffer_reserve(RustBuffer buf, uint64_t additional, RustCallStatus *out_status);
uint16_t uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_api_key_and_tls();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_bearer_token_and_tls();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_rest_client_with_sdk_token_and_tls();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_websocket_client();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_config();
uint16_t uniffi_marketdata_uniffi_checksum_func_new_websocket_client_with_endpoint();
uint16_t uniffi_marketdata_uniffi_checksum_method_futoptclient_historical();
uint16_t uniffi_marketdata_uniffi_checksum_method_futoptclient_intraday();
uint16_t uniffi_marketdata_uniffi_checksum_method_restclient_futopt();
uint16_t uniffi_marketdata_uniffi_checksum_method_restclient_stock();
uint16_t uniffi_marketdata_uniffi_checksum_method_stockclient_corporate_actions();
uint16_t uniffi_marketdata_uniffi_checksum_method_stockclient_historical();
uint16_t uniffi_marketdata_uniffi_checksum_method_stockclient_intraday();
uint16_t uniffi_marketdata_uniffi_checksum_method_stockclient_snapshot();
uint16_t uniffi_marketdata_uniffi_checksum_method_stockclient_technical();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_connect_sync();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_disconnect_sync();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_is_closed();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_is_connected();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_ping_sync();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_query_subscriptions_sync();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_subscribe_sync();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketclient_unsubscribe_sync();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_connected();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_disconnected();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_message();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_error();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnecting();
uint16_t uniffi_marketdata_uniffi_checksum_method_websocketlistener_on_reconnect_failed();
uint16_t uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new();
uint16_t uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_config();
uint16_t uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_endpoint();
uint16_t uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_full_config();
uint16_t uniffi_marketdata_uniffi_checksum_constructor_websocketclient_new_with_url();
uint32_t ffi_marketdata_uniffi_uniffi_contract_version();
#ifdef __cplusplus
}
#endif