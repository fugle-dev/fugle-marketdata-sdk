// credentials_test.go - SetCredentials changes what later connection
// attempts and requests send (#322).
//
// Run: CGO_ENABLED=1 go test -run Credentials -short .

package marketdata_uniffi

import (
	"net/http"
	"net/http/httptest"
	"reflect"
	"sync"
	"testing"
	"time"
)

func newCredentialsClient(t *testing.T, srv *authFrameServer) *StreamingClient {
	t.Helper()
	client, err := NewFugleWebSocketClient(nil,
		WithApiKey("old-key"),
		WithBaseUrl(srv.url()),
		WithReconnect(ReconnectConfig{MaxAttempts: 2, InitialDelayMs: 100, MaxDelayMs: 100}),
	)
	if err != nil {
		t.Fatalf("NewFugleWebSocketClient: %v", err)
	}
	t.Cleanup(func() { _ = client.Close() })
	return client
}

func assertConfigError(t *testing.T, err error) {
	t.Helper()
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != 1004 {
		t.Fatalf("err = %v, want ConfigError 1004", err)
	}
}

func TestSetCredentials_ReconnectSendsTheNewCredentialOfAnotherKind(t *testing.T) {
	srv := newAuthFrameServer(t)
	client := newCredentialsClient(t, srv)
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	srv.requireCredential("new-token")
	if err := client.SetCredentialsWith(WithSdkToken("new-token")); err != nil {
		t.Fatalf("SetCredentialsWith: %v", err)
	}
	time.Sleep(100 * time.Millisecond)
	if n := len(srv.authData()); n != 1 {
		t.Fatalf("%d auth frames, want no new one on the live connection", n)
	}

	srv.dropConnections()
	waitUntil(t, func() bool { return len(srv.authData()) == 2 && client.IsConnected() }, "the reconnect never authenticated")

	want := []map[string]any{{"apikey": "old-key"}, {"sdkToken": "new-token"}}
	if got := srv.authData(); !reflect.DeepEqual(got, want) {
		t.Fatalf("auth data = %v, want %v", got, want)
	}
}

func TestSetCredentials_InvalidIsConfigErrorAndKeepsTheCurrentOne(t *testing.T) {
	srv := newAuthFrameServer(t)
	client := newCredentialsClient(t, srv)

	blank := "   "
	assertConfigError(t, client.SetCredentials(CredentialsRecord{}))
	assertConfigError(t, client.SetCredentials(CredentialsRecord{BearerToken: &blank}))
	assertConfigError(t, client.SetCredentialsWith(WithApiKey("a"), WithSdkToken("b")))
	assertConfigError(t, client.SetCredentialsWith(WithSdkToken("t"), WithBaseUrl("ws://elsewhere")))
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	want := []map[string]any{{"apikey": "old-key"}}
	if got := srv.authData(); !reflect.DeepEqual(got, want) {
		t.Fatalf("auth data = %v, want %v", got, want)
	}
}

func TestRestSetCredentials_NextRequestsSendTheNewCredential(t *testing.T) {
	var mu sync.Mutex
	var headers []http.Header
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		headers = append(headers, r.Header.Clone())
		mu.Unlock()
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{}`))
	}))
	defer srv.Close()

	client, err := NewFugleRestClient(WithApiKey("old-key"), WithBaseUrl(srv.URL))
	if err != nil {
		t.Fatalf("NewFugleRestClient: %v", err)
	}
	defer client.Destroy()
	intraday := client.Stock().Intraday()

	assertConfigError(t, client.SetCredentialsWith())
	if err := client.SetCredentialsWith(WithSdkToken("new-token")); err != nil {
		t.Fatalf("SetCredentialsWith: %v", err)
	}
	if _, err := intraday.GetQuote("2330", nil); err != nil {
		t.Fatalf("GetQuote: %v", err)
	}

	mu.Lock()
	defer mu.Unlock()
	if len(headers) != 1 || headers[0].Get("X-SDK-TOKEN") != "new-token" || headers[0].Get("X-API-KEY") != "" {
		t.Fatalf("headers = %v, want only X-SDK-TOKEN new-token", headers)
	}
}

func TestSetCredentials_RejectedFirstConnectSucceedsAfterTheCredentialIsSet(t *testing.T) {
	srv := newAuthFrameServer(t)
	srv.requireCredential("new-token")
	client := newCredentialsClient(t, srv)
	if err := client.Connect(); err == nil {
		t.Fatal("Connect with the old credential succeeded")
	}

	if err := client.SetCredentialsWith(WithSdkToken("new-token")); err != nil {
		t.Fatalf("SetCredentialsWith: %v", err)
	}
	if err := client.Connect(); err != nil {
		t.Fatalf("Connect: %v", err)
	}

	want := []map[string]any{{"apikey": "old-key"}, {"sdkToken": "new-token"}}
	if got := srv.authData(); !reflect.DeepEqual(got, want) {
		t.Fatalf("auth data = %v, want %v", got, want)
	}
}

func TestSetCredentials_AfterCloseIsClientClosed(t *testing.T) {
	srv := newAuthFrameServer(t)
	client := newCredentialsClient(t, srv)
	_ = client.Close()

	for _, err := range []error{
		client.SetCredentials(CredentialsRecord{SdkToken: String("new-token")}),
		client.SetCredentialsWith(WithSdkToken("new-token")),
	} {
		if info, ok := ErrorInfoOf(err); !ok || info.Code != 2010 {
			t.Fatalf("err = %v, want ClientClosed 2010", err)
		}
	}
}
