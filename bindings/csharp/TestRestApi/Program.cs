// C# REST API 簡單測試
//
// 執行方式:
//   cd bindings/csharp
//   dotnet run --project TestRestApi

using System;
using System.Threading.Tasks;
using FugleMarketData;

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
            var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
            Console.WriteLine($"   股票代號: {quote.symbol}");
            Console.WriteLine($"   日期: {quote.date}");
            Console.WriteLine($"   收盤價: {quote.closePrice}");
            Console.WriteLine($"   漲跌: {quote.change}");
            Console.WriteLine($"   漲跌幅: {quote.changePercent}%");
            if (quote.total != null)
            {
                Console.WriteLine($"   成交量: {quote.total.tradeVolume}");
                Console.WriteLine($"   成交金額: {quote.total.tradeValue}");
            }
            Console.WriteLine();

            // 2. 取得股票基本資訊
            Console.WriteLine("2. 取得 2330 基本資訊...");
            var ticker = await client.Stock.Intraday.GetTickerAsync("2330");
            Console.WriteLine($"   名稱: {ticker.name}");
            Console.WriteLine($"   交易所: {ticker.exchange}");
            Console.WriteLine($"   漲停價: {ticker.limitUpPrice}");
            Console.WriteLine($"   跌停價: {ticker.limitDownPrice}");
            Console.WriteLine();

            // 3. 取得成交明細
            Console.WriteLine("3. 取得 2330 成交明細 (前 3 筆)...");
            var trades = await client.Stock.Intraday.GetTradesAsync("2330");
            Console.WriteLine($"   共 {trades.data.Count} 筆成交");
            for (int i = 0; i < Math.Min(3, trades.data.Count); i++)
            {
                var trade = trades.data[i];
                Console.WriteLine($"   [{i+1}] 價格: {trade.price}, 數量: {trade.size}, 時間: {trade.time}");
            }
            Console.WriteLine();

            // 4. 取得 K 線資料
            Console.WriteLine("4. 取得 2330 五分鐘 K 線 (前 3 根)...");
            var candles = await client.Stock.Intraday.GetCandlesAsync("2330", "5");
            Console.WriteLine($"   共 {candles.data.Count} 根 K 線");
            for (int i = 0; i < Math.Min(3, candles.data.Count); i++)
            {
                var candle = candles.data[i];
                Console.WriteLine($"   [{i+1}] 時間: {candle.time}, O:{candle.open} H:{candle.high} L:{candle.low} C:{candle.close} V:{candle.volume}");
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
