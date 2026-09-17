package marketdata_uniffi

import "testing"

func TestErrorInfoOf_MarketDataErrorVariant(t *testing.T) {
	info := ErrorInfo{Code: 2003, SourceKind: ErrorSourceKindClient, Message: "HTTP 404: not found"}
	err := NewMarketDataErrorApiError("HTTP 404: not found", info)

	got, ok := ErrorInfoOf(err)
	if !ok {
		t.Fatal("ErrorInfoOf did not recognise *MarketDataError")
	}
	if got.Code != 2003 || got.Message != "HTTP 404: not found" {
		t.Fatalf("got %+v, want code=2003", got)
	}
}

func TestErrorInfoOf_ClientClosedHasNoMsgField(t *testing.T) {
	info := ErrorInfo{Code: 2010, SourceKind: ErrorSourceKindClient, Message: "Client already closed"}
	err := NewMarketDataErrorClientClosed(info)

	got, ok := ErrorInfoOf(err)
	if !ok {
		t.Fatal("ErrorInfoOf did not recognise ClientClosed")
	}
	if got.Code != 2010 {
		t.Fatalf("got code=%d, want 2010", got.Code)
	}
}

func TestErrorInfoOf_UnrecognisedErrorReturnsFalse(t *testing.T) {
	if _, ok := ErrorInfoOf(nil); ok {
		t.Fatal("nil error should not carry an ErrorInfo")
	}

	if _, ok := ErrorInfoOf(&StreamError{}); !ok {
		t.Fatal("a zero StreamError is still a recognised type")
	}
	if _, ok := ErrorInfoOf(errUnrelated{}); ok {
		t.Fatal("unrelated error type should not carry an ErrorInfo")
	}
}

type errUnrelated struct{}

func (errUnrelated) Error() string { return "unrelated" }
