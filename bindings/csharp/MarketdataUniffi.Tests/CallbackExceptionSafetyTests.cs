using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;
using System.Collections.Generic;
using FugleMarketData;

namespace MarketdataUniffi.Tests;

/// <summary>
/// A user listener that throws must not crash the process nor kill the
/// event stream (#83): the adapter catches it and reports it to the
/// listener's own OnError, throttled.
/// </summary>
[TestClass]
public class CallbackExceptionSafetyTests
{
    // ========== ReportThrottle ==========

    [TestMethod]
    public void Throttle_FirstOccurrenceReportsImmediatelyWithCountOne()
    {
        var now = TimeSpan.Zero;
        var throttle = new ReportThrottle(() => now);

        Assert.AreEqual((ulong?)1, throttle.Record());
    }

    [TestMethod]
    public void Throttle_SecondOccurrenceWithinIntervalIsSuppressed()
    {
        var now = TimeSpan.Zero;
        var throttle = new ReportThrottle(() => now);

        Assert.AreEqual((ulong?)1, throttle.Record());

        now += TimeSpan.FromMilliseconds(500);
        Assert.IsNull(throttle.Record());
    }

    [TestMethod]
    public void Throttle_AfterIntervalReportsAccumulatedCount()
    {
        var now = TimeSpan.Zero;
        var throttle = new ReportThrottle(() => now);

        Assert.AreEqual((ulong?)1, throttle.Record());

        // Two more within the interval, both suppressed but counted.
        now += TimeSpan.FromMilliseconds(100);
        Assert.IsNull(throttle.Record());
        now += TimeSpan.FromMilliseconds(400);
        Assert.IsNull(throttle.Record());

        // A third occurrence right at the interval boundary reports the
        // three accumulated since the first report.
        now += TimeSpan.FromMilliseconds(500);
        Assert.AreEqual((ulong?)3, throttle.Record());
    }

    // ========== WebSocketListenerAdapter ==========

    [TestMethod]
    public void ThrowingOnMessage_IsReportedAsCallbackFailedToOnError()
    {
        var listener = new RecordingListener { ThrowOn = nameof(RecordingListener.OnMessage) };
        var adapter = new WebSocketListenerAdapter(listener);

        adapter.OnMessage(new uniffi.marketdata_uniffi.StreamMessage(
            raw: "{}", @event: "data", channel: "trades", symbol: "2330",
            id: null, dataJson: null, errorCode: null, errorMessage: null));

        Assert.AreEqual(1, listener.Errors.Count);
        var error = listener.Errors[0];
        Assert.AreEqual(3004, error.code);
        Assert.AreEqual(uniffi.marketdata_uniffi.ErrorSourceKind.Client, error.sourceKind);
        StringAssert.Contains(error.message, "OnMessage");
        StringAssert.Contains(error.message, typeof(InvalidOperationException).FullName);
        StringAssert.Contains(error.message, "boom");
        Assert.IsNull(error.status);
        Assert.IsNull(error.body);
        Assert.IsNull(error.requestId);
        Assert.AreEqual(0, error.headers.Count);
    }

    [TestMethod]
    public void SubsequentCallsStillReachTheListenerAfterAThrow()
    {
        var listener = new RecordingListener { ThrowOn = nameof(RecordingListener.OnMessage) };
        var adapter = new WebSocketListenerAdapter(listener);

        adapter.OnMessage(new uniffi.marketdata_uniffi.StreamMessage(
            raw: "{}", @event: "data", channel: null, symbol: null,
            id: null, dataJson: null, errorCode: null, errorMessage: null));

        adapter.OnConnected();
        adapter.OnReconnecting(1);

        Assert.AreEqual(1, listener.Connected);
        Assert.AreEqual((uint?)1, listener.LastReconnectingAttempt);
    }

    [TestMethod]
    public void ThrowingOnErrorDoesNotPropagateOrRecurse()
    {
        var listener = new RecordingListener
        {
            ThrowOn = nameof(RecordingListener.OnMessage),
            ThrowOnError = true,
        };
        var adapter = new WebSocketListenerAdapter(listener);

        // Must not throw out of the adapter.
        adapter.OnMessage(new uniffi.marketdata_uniffi.StreamMessage(
            raw: "{}", @event: "data", channel: null, symbol: null,
            id: null, dataJson: null, errorCode: null, errorMessage: null));

        // OnError was invoked exactly once (the callback-failure report);
        // its own throw must not trigger another report/recursion.
        Assert.AreEqual(1, listener.OnErrorCallCount);
    }

    [TestMethod]
    public void SdkOnErrorPassthroughStillWorks()
    {
        var listener = new RecordingListener();
        var adapter = new WebSocketListenerAdapter(listener);

        var sdkError = new uniffi.marketdata_uniffi.ErrorInfo(
            code: 2001,
            sourceKind: uniffi.marketdata_uniffi.ErrorSourceKind.Network,
            message: "connection reset",
            status: null,
            body: null,
            requestId: null,
            headers: new Dictionary<string, string>());

        adapter.OnError(sdkError);

        Assert.AreEqual(1, listener.Errors.Count);
        Assert.AreEqual(2001, listener.Errors[0].code);
        Assert.AreEqual("connection reset", listener.Errors[0].message);
    }

    /// <summary>Listener stub that records calls and can be made to throw.</summary>
    private sealed class RecordingListener : IWebSocketListener
    {
        public string? ThrowOn { get; set; }
        public bool ThrowOnError { get; set; }
        public int Connected { get; private set; }
        public int OnErrorCallCount { get; private set; }
        public uint? LastReconnectingAttempt { get; private set; }
        public List<uniffi.marketdata_uniffi.ErrorInfo> Errors { get; } = new();

        private void MaybeThrow(string methodName)
        {
            if (ThrowOn == methodName)
            {
                throw new InvalidOperationException("boom");
            }
        }

        public void OnConnected()
        {
            MaybeThrow(nameof(OnConnected));
            Connected++;
        }

        public void OnAuthenticated(string? dataJson) => MaybeThrow(nameof(OnAuthenticated));

        public void OnUnauthenticated(string? dataJson) => MaybeThrow(nameof(OnUnauthenticated));

        public void OnDisconnected(bool willReconnect) => MaybeThrow(nameof(OnDisconnected));

        public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) => MaybeThrow(nameof(OnMessage));

        public void OnError(uniffi.marketdata_uniffi.ErrorInfo error)
        {
            OnErrorCallCount++;
            if (ThrowOnError)
            {
                throw new InvalidOperationException("OnError itself blew up");
            }
            Errors.Add(error);
        }

        public void OnReconnecting(uint attempt)
        {
            MaybeThrow(nameof(OnReconnecting));
            LastReconnectingAttempt = attempt;
        }

        public void OnReconnectFailed(uint attempts) => MaybeThrow(nameof(OnReconnectFailed));

        public void OnMessagesDropped(ulong count) => MaybeThrow(nameof(OnMessagesDropped));
    }
}
