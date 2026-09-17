// ws_subscribe_frame_test.go - subscribe and unsubscribe follow the client's
// endpoint (#123).
//
// The FutOpt endpoint takes FutOpt channels and the after-hours session; the
// Stock endpoint rejects after-hours with 1005.
//
// Run: CGO_ENABLED=1 go test -run SubscribeFrame -short .

package marketdata_uniffi

import (
	"encoding/json"
	"reflect"
	"testing"
	"time"
)

func TestSubscribeFrame_FutOptEndpointSendsAfterHours(t *testing.T) {
	srv := newAuthFrameServer(t)
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("the-key"), WithEndpoint(WebSocketEndpointFutOpt), WithBaseUrl(srv.url()))
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	assertErrorCode(t, client.Subscribe("indices", "TXFE6"), 1005)
	if err := client.Subscribe("books", "TXFE6", WithAfterHours(true)); err != nil {
		t.Fatalf("Subscribe after hours: %v", err)
	}
	if err := client.Subscribe("trades", "TXFE6"); err != nil {
		t.Fatalf("Subscribe: %v", err)
	}
	if err := client.Unsubscribe("books", "TXFE6", WithAfterHours(true)); err != nil {
		t.Fatalf("Unsubscribe after hours: %v", err)
	}

	want := []map[string]any{
		{"event": "subscribe", "data": map[string]any{"channel": "books", "symbol": "TXFE6", "afterHours": true}},
		{"event": "subscribe", "data": map[string]any{"channel": "trades", "symbol": "TXFE6"}},
		{"event": "unsubscribe", "data": map[string]any{"id": "books:TXFE6:afterhours"}},
	}
	var got []map[string]any
	for deadline := time.Now().Add(10 * time.Second); time.Now().Before(deadline); time.Sleep(20 * time.Millisecond) {
		if got = decodeFrames(t, srv.otherFrames()); len(got) >= len(want) {
			break
		}
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("frames = %v, want %v", got, want)
	}
}

func TestSubscribeFrame_StockEndpointRejectsAfterHours(t *testing.T) {
	client, err := NewFugleWebSocketClient(nil, WithApiKey("the-key"))
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	defer client.Close()

	for _, afterHours := range []bool{true, false} {
		assertErrorCode(t, client.Subscribe("trades", "2330", WithAfterHours(afterHours)), 1005)
		assertErrorCode(t, client.Unsubscribe("trades", "2330", WithAfterHours(afterHours)), 1005)
	}
}

func assertErrorCode(t *testing.T, err error, code int32) {
	t.Helper()
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != code {
		t.Fatalf("error = %v, want code %d", err, code)
	}
}

func decodeFrames(t *testing.T, frames []string) []map[string]any {
	t.Helper()
	decoded := make([]map[string]any, 0, len(frames))
	for _, frame := range frames {
		var m map[string]any
		if err := json.Unmarshal([]byte(frame), &m); err != nil {
			t.Fatalf("frame %q: %v", frame, err)
		}
		decoded = append(decoded, m)
	}
	return decoded
}
