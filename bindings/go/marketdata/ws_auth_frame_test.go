// ws_auth_frame_test.go - the auth frame carries each credential in its own
// field (#91).
//
// The server reads `apikey`, `token` or `sdkToken` and rejects a frame with
// more than one. Bearer and SDK tokens used to be refused outright.
//
// Run: CGO_ENABLED=1 go test -run AuthFrame -short .

package marketdata_uniffi

import (
	"bufio"
	"crypto/sha1"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strings"
	"sync"
	"testing"
)

func TestWebSocketAuthFrame_CredentialInItsField(t *testing.T) {
	cases := []struct {
		name     string
		option   Option
		expected map[string]any
	}{
		{"WithApiKey", WithApiKey("the-key"), map[string]any{"apikey": "the-key"}},
		{"WithBearerToken", WithBearerToken("the-token"), map[string]any{"token": "the-token"}},
		{"WithSdkToken", WithSdkToken("the-sdk-token"), map[string]any{"sdkToken": "the-sdk-token"}},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			srv := newAuthFrameServer(t)

			client, err := NewFugleWebSocketClient(nil, tc.option, WithBaseUrl(srv.url()))
			if err != nil {
				t.Fatalf("NewFugleWebSocketClient: %v", err)
			}
			if err := client.Connect(); err != nil {
				t.Fatalf("Connect: %v", err)
			}
			_ = client.Close()

			got := srv.authData()
			if len(got) != 1 || !reflect.DeepEqual(got[0], tc.expected) {
				t.Fatalf("auth data = %v, want [%v]", got, tc.expected)
			}
		})
	}
}

// authFrameServer is a loopback WebSocket server on the standard library
// alone: it records the `data` of every auth frame and acks it, and records
// every other text frame as is. dropConnections cuts the open connections
// without a Close frame.
type authFrameServer struct {
	srv    *httptest.Server
	mu     sync.Mutex
	auth   []map[string]any
	others []string
	conns  []net.Conn
}

func newAuthFrameServer(t *testing.T) *authFrameServer {
	t.Helper()
	s := &authFrameServer{}
	s.srv = httptest.NewServer(http.HandlerFunc(s.serve))
	t.Cleanup(s.srv.Close)
	return s
}

func (s *authFrameServer) url() string {
	return "ws" + strings.TrimPrefix(s.srv.URL, "http")
}

// otherFrames are the text frames other than auth, in arrival order.
func (s *authFrameServer) otherFrames() []string {
	s.mu.Lock()
	defer s.mu.Unlock()
	return append([]string(nil), s.others...)
}

func (s *authFrameServer) authData() []map[string]any {
	s.mu.Lock()
	defer s.mu.Unlock()
	return append([]map[string]any(nil), s.auth...)
}

// dropConnections cuts every open connection at the transport, as a network
// failure would.
func (s *authFrameServer) dropConnections() {
	s.mu.Lock()
	conns := s.conns
	s.conns = nil
	s.mu.Unlock()
	for _, conn := range conns {
		_ = conn.Close()
	}
}

func (s *authFrameServer) serve(w http.ResponseWriter, r *http.Request) {
	sum := sha1.Sum([]byte(r.Header.Get("Sec-WebSocket-Key") + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
	conn, rw, err := w.(http.Hijacker).Hijack()
	if err != nil {
		return
	}
	defer conn.Close()
	s.mu.Lock()
	s.conns = append(s.conns, conn)
	s.mu.Unlock()
	_, _ = rw.WriteString("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" +
		"Sec-WebSocket-Accept: " + base64.StdEncoding.EncodeToString(sum[:]) + "\r\n\r\n")
	if rw.Flush() != nil {
		return
	}

	for {
		opcode, payload, err := readFrame(rw.Reader)
		if err != nil {
			return
		}
		switch opcode {
		case 0x8: // close
			_ = writeFrame(conn, 0x8, payload)
			return
		case 0x9: // ping
			_ = writeFrame(conn, 0xA, payload)
		case 0x1: // text
			var frame struct {
				Event string         `json:"event"`
				Data  map[string]any `json:"data"`
			}
			if json.Unmarshal(payload, &frame) != nil {
				continue
			}
			if frame.Event != "auth" {
				s.mu.Lock()
				s.others = append(s.others, string(payload))
				s.mu.Unlock()
				continue
			}
			s.mu.Lock()
			s.auth = append(s.auth, frame.Data)
			s.mu.Unlock()
			_ = writeFrame(conn, 0x1, []byte(`{"event":"authenticated","data":{"message":"Authenticated successfully"}}`))
		}
	}
}

func readFrame(r *bufio.Reader) (byte, []byte, error) {
	var head [2]byte
	if _, err := io.ReadFull(r, head[:]); err != nil {
		return 0, nil, err
	}
	length := uint64(head[1] & 0x7f)
	switch length {
	case 126:
		var ext [2]byte
		if _, err := io.ReadFull(r, ext[:]); err != nil {
			return 0, nil, err
		}
		length = uint64(binary.BigEndian.Uint16(ext[:]))
	case 127:
		var ext [8]byte
		if _, err := io.ReadFull(r, ext[:]); err != nil {
			return 0, nil, err
		}
		length = binary.BigEndian.Uint64(ext[:])
	}
	var mask [4]byte
	masked := head[1]&0x80 != 0
	if masked {
		if _, err := io.ReadFull(r, mask[:]); err != nil {
			return 0, nil, err
		}
	}
	payload := make([]byte, length)
	if _, err := io.ReadFull(r, payload); err != nil {
		return 0, nil, err
	}
	if masked {
		for i := range payload {
			payload[i] ^= mask[i%4]
		}
	}
	return head[0] & 0x0f, payload, nil
}

func writeFrame(conn net.Conn, opcode byte, payload []byte) error {
	frame := []byte{0x80 | opcode}
	if len(payload) < 126 {
		frame = append(frame, byte(len(payload)))
	} else {
		frame = append(frame, 126, byte(len(payload)>>8), byte(len(payload)))
	}
	_, err := conn.Write(append(frame, payload...))
	return err
}
