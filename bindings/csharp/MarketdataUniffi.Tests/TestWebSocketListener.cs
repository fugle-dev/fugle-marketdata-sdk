namespace MarketdataUniffi.Tests;

/// <summary>A listener that ignores every event, for tests that only build or connect a client.</summary>
internal sealed class TestWebSocketListener : FugleMarketData.IWebSocketListener
{
    public void OnConnected() { }
    public void OnAuthenticated(string? dataJson) { }
    public void OnUnauthenticated(string? dataJson) { }
    public void OnDisconnected(bool willReconnect) { }
    public void OnMessage(uniffi.marketdata_uniffi.StreamMessage message) { }
    public void OnError(uniffi.marketdata_uniffi.ErrorInfo error) { }
    public void OnReconnecting(uint attempt) { }
    public void OnReconnectFailed(uint attempts) { }
    public void OnMessagesDropped(ulong count) { }
}
