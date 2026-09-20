// C# REST API 簡單測試
//
// 執行方式:
//   cd bindings/csharp
//   dotnet run --project TestRestApi

using System;
using System.Text.Json;
using System.Threading.Tasks;
using FugleMarketData;
using StockTradesParams = uniffi.marketdata_uniffi.StockTradesParams;
using StockCandlesParams = uniffi.marketdata_uniffi.StockCandlesParams;

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

        Console.WriteLine("=== C# REST API 測試 ===\n");

        using var client = new RestClient(apiKey);

        try
        {
            // 1. 取得股票報價
            Console.WriteLine("1. 取得 2330 (台積電) 報價...");
            var quote = JsonDocument.Parse(await client.Stock.Intraday.GetQuoteAsync("2330")).RootElement;
            Console.WriteLine($"   股票代號: {quote.GetProperty("symbol")}");
            Console.WriteLine($"   日期: {quote.GetProperty("date")}");
            Console.WriteLine($"   收盤價: {quote.GetProperty("closePrice")}");
            Console.WriteLine($"   漲跌: {quote.GetProperty("change")}");
            Console.WriteLine($"   漲跌幅: {quote.GetProperty("changePercent")}%");
            if (quote.TryGetProperty("total", out var total))
            {
                Console.WriteLine($"   成交量: {total.GetProperty("tradeVolume")}");
                Console.WriteLine($"   成交金額: {total.GetProperty("tradeValue")}");
            }
            Console.WriteLine();

            // 2. 取得股票基本資訊
            Console.WriteLine("2. 取得 2330 基本資訊...");
            var ticker = JsonDocument.Parse(await client.Stock.Intraday.GetTickerAsync("2330")).RootElement;
            Console.WriteLine($"   名稱: {ticker.GetProperty("name")}");
            Console.WriteLine($"   交易所: {ticker.GetProperty("exchange")}");
            Console.WriteLine($"   漲停價: {ticker.GetProperty("limitUpPrice")}");
            Console.WriteLine($"   跌停價: {ticker.GetProperty("limitDownPrice")}");
            Console.WriteLine();

            // 3. 取得成交明細
            Console.WriteLine("3. 取得 2330 成交明細 (前 3 筆)...");
            var trades = JsonDocument.Parse(
                await client.Stock.Intraday.GetTradesAsync("2330", new StockTradesParams(limit: 3))).RootElement;
            var tradesData = trades.GetProperty("data");
            Console.WriteLine($"   共 {tradesData.GetArrayLength()} 筆成交");
            var i = 0;
            foreach (var trade in tradesData.EnumerateArray())
            {
                Console.WriteLine($"   [{++i}] 價格: {trade.GetProperty("price")}, 數量: {trade.GetProperty("size")}, 時間: {trade.GetProperty("time")}");
            }
            Console.WriteLine();

            // 4. 取得 K 線資料
            Console.WriteLine("4. 取得 2330 五分鐘 K 線...");
            var candles = JsonDocument.Parse(
                await client.Stock.Intraday.GetCandlesAsync("2330", new StockCandlesParams(timeframe: "5"))).RootElement;
            var candlesData = candles.GetProperty("data");
            Console.WriteLine($"   共 {candlesData.GetArrayLength()} 根 K 線");
            i = 0;
            foreach (var candle in candlesData.EnumerateArray())
            {
                if (++i > 3) break;
                Console.WriteLine($"   [{i}] 時間: {candle.GetProperty("date")}, O:{candle.GetProperty("open")} H:{candle.GetProperty("high")} L:{candle.GetProperty("low")} C:{candle.GetProperty("close")} V:{candle.GetProperty("volume")}");
            }
            Console.WriteLine();

            Console.WriteLine("=== 測試完成 ===");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"錯誤: {ex.Message}");
            Environment.Exit(1);
        }
    }
}
