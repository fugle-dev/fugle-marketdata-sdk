// passthrough_test.go - the server's JSON reaches the caller untouched.
//
// This replaces response_compatibility_test.go, which used reflection to
// assert that the mirrored `Quote` struct had a `Symbol` field. That is
// tautological — the struct's own definition guaranteed it — and it never
// looked at a real response, so it happily passed while the mirror was
// missing 22 fields. Those mirrors are gone; REST methods now hand back the
// server's JSON verbatim.
//
// Run: CGO_ENABLED=1 go test -run Passthrough -short .

package marketdata_uniffi

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"reflect"
	"sync"
	"testing"
)

// quote2330 is a real GET /stock/intraday/quote/2330 body, captured 2026-09-16.
const quote2330 = `{
  "date": "2026-09-16",
  "type": "EQUITY",
  "exchange": "TWSE",
  "market": "TSE",
  "symbol": "2330",
  "name": "台積電",
  "referencePrice": 2380,
  "previousClose": 2385,
  "openPrice": 2375,
  "highPrice": 2385,
  "lowPrice": 2375,
  "closePrice": 2385,
  "avgPrice": 2378.51,
  "change": 5,
  "changePercent": 0.21,
  "amplitude": 0.42,
  "lastPrice": 2385,
  "lastSize": 1,
  "bids": [{"price": 2380, "size": 185}, {"price": 2375, "size": 1407}],
  "asks": [{"price": 2385, "size": 199}, {"price": 2390, "size": 556}],
  "total": {
    "tradeValue": 14566025000,
    "tradeVolume": 6124,
    "tradeVolumeAtBid": 1962,
    "tradeVolumeAtAsk": 2686,
    "transaction": 1922,
    "time": 1789525853959140
  },
  "lastTrade": {
    "bid": 2380, "ask": 2385, "price": 2385, "size": 1,
    "time": 1789525853959140, "serial": 6132837
  },
  "isContinuous": true,
  "serial": 6152257,
  "lastUpdated": 1789525882301247
}`

// serveJSON starts a loopback server returning body on every request and
// returns a client pointed at it.
func serveJSON(t *testing.T, body string) *RestClient {
	t.Helper()

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(body))
	}))
	t.Cleanup(srv.Close)

	// base_url takes the host only — a "/v1.0" segment is rejected.
	client, err := NewFugleRestClient(WithApiKey("test-key"), WithBaseUrl(srv.URL))
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	t.Cleanup(client.Destroy)

	return client
}

// queryRecorder records the RawQuery of every request a queryServer
// received, in arrival order.
type queryRecorder struct {
	mu      sync.Mutex
	queries []string
}

func (r *queryRecorder) record(raw string) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.queries = append(r.queries, raw)
}

// all returns every recorded query, in arrival order.
func (r *queryRecorder) all() []string {
	r.mu.Lock()
	defer r.mu.Unlock()
	return append([]string(nil), r.queries...)
}

// queryServer starts a loopback server returning body on every request,
// recording each request's RawQuery, and returns a client pointed at it
// plus the recorder.
func queryServer(t *testing.T, body string) (*RestClient, *queryRecorder) {
	t.Helper()
	rec := &queryRecorder{}

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		rec.record(r.URL.RawQuery)
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(body))
	}))
	t.Cleanup(srv.Close)

	client, err := NewFugleRestClient(WithApiKey("test-key"), WithBaseUrl(srv.URL))
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	t.Cleanup(client.Destroy)

	return client, rec
}

// assertQuery parses raw as a query string and compares it against want,
// as a set of key -> values (order-independent).
func assertQuery(t *testing.T, raw string, want url.Values) {
	t.Helper()
	got, err := url.ParseQuery(raw)
	if err != nil {
		t.Fatalf("ParseQuery(%q): %v", raw, err)
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("query = %v, want %v", got, want)
	}
}

func TestQueryParams_TradesSendsOddLotAndLimit(t *testing.T) {
	client, rec := queryServer(t, `{}`)
	if _, err := client.Stock().Intraday().GetTrades("2330", &StockTradesParams{OddLot: Bool(true), Limit: Uint32(5)}); err != nil {
		t.Fatalf("GetTrades failed: %v", err)
	}
	assertQuery(t, rec.all()[0], url.Values{"type": {"oddlot"}, "limit": {"5"}})
}

func TestQueryParams_MoversSendsDirectionAndChange(t *testing.T) {
	client, rec := queryServer(t, `{}`)
	if _, err := client.Stock().Snapshot().GetMovers("TSE", "up", "percent", nil); err != nil {
		t.Fatalf("GetMovers failed: %v", err)
	}
	assertQuery(t, rec.all()[0], url.Values{"direction": {"up"}, "change": {"percent"}})
}

func TestQueryParams_CapitalChangesWithExchangeIsInvalidParameter(t *testing.T) {
	client, rec := queryServer(t, `{}`)
	_, err := client.Stock().CorporateActions().GetCapitalChanges(&CorporateActionsParams{Exchange: String("TWSE")})
	if err == nil {
		t.Fatal("GetCapitalChanges with exchange should have errored")
	}
	info, ok := ErrorInfoOf(err)
	if !ok || info.Code != 1005 {
		t.Fatalf("error = %v (info=%+v, ok=%v), want code 1005", err, info, ok)
	}
	if queries := rec.all(); len(queries) != 0 {
		t.Fatalf("request should not have reached the server, got queries %v", queries)
	}
}

func TestQueryParams_NilParamsSendsEmptyQuery(t *testing.T) {
	client, rec := queryServer(t, quote2330)
	if _, err := client.Stock().Intraday().GetQuote("2330", nil); err != nil {
		t.Fatalf("GetQuote failed: %v", err)
	}
	if got := rec.all()[0]; got != "" {
		t.Fatalf("query = %q, want empty", got)
	}
}

func TestQueryParams_FutOptProductsSendsTypeAndSession(t *testing.T) {
	client, rec := queryServer(t, `{}`)
	if _, err := client.Futopt().Intraday().GetProducts("F", &FutOptProductsParams{AfterHours: Bool(true)}); err != nil {
		t.Fatalf("GetProducts failed: %v", err)
	}
	assertQuery(t, rec.all()[0], url.Values{"type": {"FUTURE"}, "session": {"AFTERHOURS"}})
}

func decode(t *testing.T, body string) map[string]any {
	t.Helper()

	var m map[string]any
	if err := json.Unmarshal([]byte(body), &m); err != nil {
		t.Fatalf("Failed to decode response: %v", err)
	}
	return m
}

func TestPassthrough_ResponseMatchesWhatTheServerSent(t *testing.T) {
	got, err := serveJSON(t, quote2330).Stock().Intraday().GetQuote("2330", nil)
	if err != nil {
		t.Fatalf("GetQuote failed: %v", err)
	}

	want := decode(t, quote2330)
	have := decode(t, got)

	if len(have) != len(want) {
		t.Fatalf("field count differs: got %d, want %d", len(have), len(want))
	}
	for key, wantVal := range want {
		haveVal, ok := have[key]
		if !ok {
			t.Errorf("field %q was dropped", key)
			continue
		}
		wj, _ := json.Marshal(wantVal)
		hj, _ := json.Marshal(haveVal)
		if string(wj) != string(hj) {
			t.Errorf("field %q changed: got %s, want %s", key, hj, wj)
		}
	}
}

func TestPassthrough_ReferencePriceIsTheBasisForChange(t *testing.T) {
	got, err := serveJSON(t, quote2330).Stock().Intraday().GetQuote("2330", nil)
	if err != nil {
		t.Fatalf("GetQuote failed: %v", err)
	}
	q := decode(t, got)

	ref, last, change := q["referencePrice"].(float64), q["lastPrice"].(float64), q["change"].(float64)
	if ref != 2380 {
		t.Errorf("referencePrice: got %v, want 2380", ref)
	}
	if last-ref != change {
		t.Errorf("lastPrice - referencePrice = %v, want change %v", last-ref, change)
	}
	// Deriving it from previousClose would have given the wrong answer.
	if prev := q["previousClose"].(float64); last-prev == change {
		t.Error("previousClose must not coincidentally equal the reference basis")
	}
}

func TestPassthrough_OmittedFieldsStayAbsent(t *testing.T) {
	got, err := serveJSON(t, quote2330).Stock().Intraday().GetQuote("2330", nil)
	if err != nil {
		t.Fatalf("GetQuote failed: %v", err)
	}
	q := decode(t, got)

	// The server sent only isContinuous. The others used to be materialised as
	// false, which callers could not tell apart from a real false.
	if q["isContinuous"] != true {
		t.Error("isContinuous should be true")
	}
	for _, key := range []string{"isOpen", "isClose", "isTrial", "isLimitUpPrice", "tradingHalt"} {
		if _, present := q[key]; present {
			t.Errorf("field %q should be absent, the server did not send it", key)
		}
	}
}

func TestPassthrough_UnknownFieldStillReachesTheCaller(t *testing.T) {
	body := `{"symbol":"2330","someFieldAddedLater":{"nested":[1,2]}}`
	got, err := serveJSON(t, body).Stock().Intraday().GetQuote("2330", nil)
	if err != nil {
		t.Fatalf("GetQuote failed: %v", err)
	}

	q := decode(t, got)
	if _, present := q["someFieldAddedLater"]; !present {
		t.Error("a field this SDK has never heard of must still reach the caller")
	}
}

func TestPassthrough_TickersKeepsTheEnvelope(t *testing.T) {
	body := `{"date":"2026-09-16","type":"EQUITY","exchange":"TWSE","market":"TSE",
	          "data":[{"symbol":"2330","name":"台積電"}]}`
	got, err := serveJSON(t, body).Stock().Intraday().GetTickers("EQUITY", nil)
	if err != nil {
		t.Fatalf("GetTickers failed: %v", err)
	}
	env := decode(t, got)

	// Earlier releases returned just `data`, losing the sibling metadata and
	// diverging from the official SDK.
	if env["exchange"] != "TWSE" {
		t.Errorf("envelope metadata lost: exchange = %v", env["exchange"])
	}
	data, ok := env["data"].([]any)
	if !ok || len(data) != 1 {
		t.Fatalf("data should be a one-element array, got %v", env["data"])
	}
}
