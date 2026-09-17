package tw.com.fugle.marketdata;

import tw.com.fugle.marketdata.generated.*;
import org.junit.jupiter.api.*;
import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;

/**
 * A user listener that throws must not crash the process nor kill the
 * event stream (#83): {@link FugleWebSocketClient.SafeListener} catches it
 * and reports it to the listener's own onError, throttled.
 */
public class CallbackExceptionSafetyTest {

    // ========== ReportThrottle ==========

    @Test
    @DisplayName("first occurrence reports immediately with count 1")
    void throttleFirstOccurrenceReportsImmediately() {
        AtomicLong now = new AtomicLong(0);
        ReportThrottle throttle = new ReportThrottle(now::get);

        assertEquals(Long.valueOf(1), throttle.record());
    }

    @Test
    @DisplayName("second occurrence within 1s is suppressed")
    void throttleSecondOccurrenceWithinIntervalIsSuppressed() {
        AtomicLong now = new AtomicLong(0);
        ReportThrottle throttle = new ReportThrottle(now::get);

        assertEquals(Long.valueOf(1), throttle.record());

        now.addAndGet(500);
        assertNull(throttle.record());
    }

    @Test
    @DisplayName("after 1s the next report carries the accumulated count")
    void throttleAfterIntervalReportsAccumulatedCount() {
        AtomicLong now = new AtomicLong(0);
        ReportThrottle throttle = new ReportThrottle(now::get);

        assertEquals(Long.valueOf(1), throttle.record());

        now.addAndGet(100);
        assertNull(throttle.record());
        now.addAndGet(400);
        assertNull(throttle.record());

        now.addAndGet(500);
        assertEquals(Long.valueOf(3), throttle.record());
    }

    // ========== SafeListener ==========

    private static StreamMessage message() {
        return new StreamMessage("{}", "data", "trades", "2330", null, null, null, null);
    }

    @Test
    @DisplayName("a throwing onMessage is reported as CALLBACK_FAILED to onError")
    void throwingOnMessageIsReportedAsCallbackFailedToOnError() {
        RecordingListener listener = new RecordingListener();
        listener.throwOn = "onMessage";
        FugleWebSocketClient.SafeListener safe = new FugleWebSocketClient.SafeListener(listener);

        safe.onMessage(message());

        assertEquals(1, listener.errors.size());
        ErrorInfo error = listener.errors.get(0);
        assertEquals(Integer.valueOf(3004), error.code());
        assertEquals(ErrorSourceKind.CLIENT, error.sourceKind());
        assertTrue(error.message().contains("onMessage"));
        assertTrue(error.message().contains(IllegalStateException.class.getName()));
        assertTrue(error.message().contains("boom"));
        assertNull(error.status());
        assertNull(error.body());
        assertNull(error.requestId());
        assertEquals(Collections.emptyMap(), error.headers());
    }

    @Test
    @DisplayName("subsequent calls still reach the listener after a throw")
    void subsequentCallsStillReachTheListenerAfterAThrow() {
        RecordingListener listener = new RecordingListener();
        listener.throwOn = "onMessage";
        FugleWebSocketClient.SafeListener safe = new FugleWebSocketClient.SafeListener(listener);

        safe.onMessage(message());
        safe.onConnected();
        safe.onReconnecting(1);

        assertEquals(1, listener.connected);
        assertEquals(Integer.valueOf(1), listener.lastReconnectingAttempt);
    }

    @Test
    @DisplayName("onError throwing does not propagate out of the wrapper and does not re-call onError")
    void throwingOnErrorDoesNotPropagateOrRecurse() {
        RecordingListener listener = new RecordingListener();
        listener.throwOn = "onMessage";
        listener.throwOnError = true;
        FugleWebSocketClient.SafeListener safe = new FugleWebSocketClient.SafeListener(listener);

        // Must not throw out of the wrapper.
        assertDoesNotThrow(() -> safe.onMessage(message()));

        // onError was invoked exactly once (the callback-failure report);
        // its own throw must not trigger another report/recursion.
        assertEquals(1, listener.onErrorCallCount);
    }

    @Test
    @DisplayName("a normal SDK error still reaches onError unchanged")
    void sdkOnErrorPassthroughStillWorks() {
        RecordingListener listener = new RecordingListener();
        FugleWebSocketClient.SafeListener safe = new FugleWebSocketClient.SafeListener(listener);

        ErrorInfo sdkError = new ErrorInfo(
                2001, ErrorSourceKind.NETWORK, "connection reset",
                null, null, null, Collections.emptyMap());

        safe.onError(sdkError);

        assertEquals(1, listener.errors.size());
        assertEquals(Integer.valueOf(2001), listener.errors.get(0).code());
        assertEquals("connection reset", listener.errors.get(0).message());
    }

    /** Listener stub that records calls and can be made to throw. */
    private static final class RecordingListener implements WebSocketListener {
        String throwOn;
        boolean throwOnError;
        int connected;
        int onErrorCallCount;
        Integer lastReconnectingAttempt;
        List<ErrorInfo> errors = new ArrayList<>();

        private void maybeThrow(String methodName) {
            if (methodName.equals(throwOn)) {
                throw new IllegalStateException("boom");
            }
        }

        @Override
        public void onConnected() {
            maybeThrow("onConnected");
            connected++;
        }

        @Override
        public void onAuthenticated(String dataJson) {
            maybeThrow("onAuthenticated");
        }

        @Override
        public void onUnauthenticated(String dataJson) {
            maybeThrow("onUnauthenticated");
        }

        @Override
        public void onDisconnected(Boolean willReconnect) {
            maybeThrow("onDisconnected");
        }

        @Override
        public void onMessage(StreamMessage message) {
            maybeThrow("onMessage");
        }

        @Override
        public void onError(ErrorInfo error) {
            onErrorCallCount++;
            if (throwOnError) {
                throw new IllegalStateException("onError itself blew up");
            }
            errors.add(error);
        }

        @Override
        public void onReconnecting(Integer attempt) {
            maybeThrow("onReconnecting");
            lastReconnectingAttempt = attempt;
        }

        @Override
        public void onReconnectFailed(Integer attempts) {
            maybeThrow("onReconnectFailed");
        }

        @Override
        public void onMessagesDropped(Long count) {
            maybeThrow("onMessagesDropped");
        }
    }
}
