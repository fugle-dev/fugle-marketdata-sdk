// C# WebSocket 簡單測試
//
// 執行方式:
//   cd bindings/csharp
//   dotnet run --project TestWebSocket

using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using FugleMarketData;
using uniffi.marketdata_uniffi;

/// <summary>
/// WebSocket 事件監聽器
/// </summary>
class MyListener : IWebSocketListener
{
    public List<StreamMessage> ReceivedMessages { get; } = new();
    public bool IsConnected { get; private set; }

    public void OnConnected()
    {
        IsConnected = true;
        Console.WriteLine("✓ 已連線!");
    }

    public void OnDisconnected()
    {
        IsConnected = false;
        Console.WriteLine("✓ 已斷線!");
    }

    public void OnMessage(StreamMessage message)
    {
        ReceivedMessages.Add(message);

        var eventType = message.@event;
        var channel = message.channel ?? "?";
        var symbol = message.symbol;

        // 如果頂層沒有 symbol，嘗試從 dataJson 解析
        string? dataSymbol = null;
        JsonDocument? doc = null;

        if (!string.IsNullOrEmpty(message.dataJson))
        {
            try
            {
                doc = JsonDocument.Parse(message.dataJson);
                var root = doc.RootElement;

                // 嘗試從 data 取得 symbol
                if (root.TryGetProperty("symbol", out var symProp))
                    dataSymbol = symProp.GetString();
            }
            catch
            {
                // 忽略解析錯誤
            }
        }

        var displaySymbol = symbol ?? dataSymbol ?? "?";
        Console.WriteLine($"✓ 收到訊息 event={eventType}, channel={channel}, symbol={displaySymbol}");

        // 顯示資料詳情
        if (doc != null)
        {
            try
            {
                var root = doc.RootElement;

                if (root.TryGetProperty("price", out var price))
                    Console.WriteLine($"  價格: {price}");
                if (root.TryGetProperty("size", out var size))
                    Console.WriteLine($"  數量: {size}");
                if (root.TryGetProperty("time", out var time))
                    Console.WriteLine($"  時間: {time}");
                if (root.TryGetProperty("closePrice", out var closePrice))
                    Console.WriteLine($"  收盤價: {closePrice}");
                if (root.TryGetProperty("change", out var change))
                    Console.WriteLine($"  漲跌: {change}");
            }
            catch
            {
                // 忽略解析錯誤
            }
            finally
            {
                doc.Dispose();
            }
        }
    }

    public void OnError(string errorMessage)
    {
        Console.WriteLine($"✗ 錯誤: {errorMessage}");
    }

    public void OnReconnecting(uint attempt)
    {
        Console.WriteLine($"⟳ 重新連線中... (第 {attempt} 次)");
    }

    public void OnReconnectFailed(uint attempts)
    {
        Console.WriteLine($"✗ 重新連線失敗 (已嘗試 {attempts} 次)");
    }
}

class Program
{
    static async Task Main(string[] args)
    {
        // 從環境變數取得 API key
        var apiKey = Environment.GetEnvironmentVariable("FUGLE_API_KEY");
        if (string.IsNullOrEmpty(apiKey))
        {
            Console.WriteLine("請設定 FUGLE_API_KEY 環境變數");
            Console.WriteLine("  export FUGLE_API_KEY='your-api-key'");
            Environment.Exit(1);
        }

        Console.WriteLine("=== C# WebSocket 測試 ===\n");

        var listener = new MyListener();

        using var client = new FugleMarketData.WebSocketClient(apiKey, listener);

        try
        {
            // 1. 連線
            Console.WriteLine("1. 連線中...");
            await client.ConnectAsync();
            Console.WriteLine();

            // 2. 訂閱
            Console.WriteLine("2. 訂閱 2330 trades...");
            await client.SubscribeAsync("trades", "2330");
            Console.WriteLine();

            // 3. 等待訊息
            Console.WriteLine("3. 等待訊息 (30秒)...\n");
            await Task.Delay(TimeSpan.FromSeconds(30));

            // 4. 結果
            Console.WriteLine($"\n=== 總共收到 {listener.ReceivedMessages.Count} 則訊息 ===\n");

            // 5. 斷線
            Console.WriteLine("4. 斷線中...");
            await client.DisconnectAsync();

            Console.WriteLine("\n=== 測試完成 ===");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"錯誤: {ex.Message}");
            Console.WriteLine(ex.StackTrace);
            Environment.Exit(1);
        }
    }
}
