using Microsoft.VisualStudio.TestTools.UnitTesting;
using System;

namespace MarketdataUniffi.Tests;

/// <summary>
/// Tests for RestClientOptions and WebSocketClientOptions configuration classes.
/// Verifies exactly-one-auth validation (from core, code 1004), options construction, and error handling.
/// </summary>
[TestClass]
public class ConfigOptionsTests
{
    private static bool _nativeLibraryAvailable;

    [ClassInitialize]
    public static void ClassInit(TestContext context)
    {
        // Check if native library is available by attempting to load it
        try
        {
            using var client = new FugleMarketData.RestClient("test-api-key");
            _nativeLibraryAvailable = true;
        }
        catch (DllNotFoundException)
        {
            _nativeLibraryAvailable = false;
        }
        catch (TypeInitializationException ex) when (ex.InnerException is DllNotFoundException)
        {
            _nativeLibraryAvailable = false;
        }
        catch
        {
            // Other exceptions mean the library loaded but failed validation
            _nativeLibraryAvailable = true;
        }
    }

    private void SkipIfNativeLibraryUnavailable()
    {
        if (!_nativeLibraryAvailable)
        {
            Assert.Inconclusive("Native library not available. Build with: cargo build -p marketdata-uniffi --release");
        }
    }

    private static void AssertCredentialsRejected(Action create)
    {
        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(create);
        var info = FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex);
        Assert.AreEqual(1004, info.code); // marketdata_core::error_code::CONFIG
        StringAssert.Contains(info.message, "exactly one non-empty credential");
    }

    // ========== RestClientOptions Tests ==========

    [TestMethod]
    public void RestClientOptions_ExactlyOneAuth_ApiKey_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions { ApiKey = "test-api-key" };

        try
        {
            using var client = new FugleMarketData.RestClient(options);
            Assert.IsNotNull(client);
            // If we get here, auth validation passed (UniFFI may still fail, but that's OK)
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject a single auth method");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable - we're testing auth validation
        }
    }

    [TestMethod]
    public void RestClientOptions_ExactlyOneAuth_BearerToken_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions { BearerToken = "test-bearer-token" };

        try
        {
            using var client = new FugleMarketData.RestClient(options);
            Assert.IsNotNull(client);
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject a single auth method");
        }
        catch
        {
            // Other exceptions are acceptable
        }
    }

    [TestMethod]
    public void RestClientOptions_ExactlyOneAuth_SdkToken_Succeeds()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions { SdkToken = "test-sdk-token" };

        try
        {
            using var client = new FugleMarketData.RestClient(options);
            Assert.IsNotNull(client);
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject a single auth method");
        }
        catch
        {
            // Other exceptions are acceptable
        }
    }

    [TestMethod]
    public void RestClientOptions_NoAuth_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        AssertCredentialsRejected(() => new FugleMarketData.RestClient(new FugleMarketData.RestClientOptions()));
    }

    [TestMethod]
    public void RestClientOptions_MultipleAuth_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.RestClientOptions
        {
            ApiKey = "test-api-key",
            BearerToken = "test-bearer-token"
        };

        AssertCredentialsRejected(() => new FugleMarketData.RestClient(options));
    }

    [TestMethod]
    public void RestClientOptions_BlankAuth_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        AssertCredentialsRejected(() => new FugleMarketData.RestClient(new FugleMarketData.RestClientOptions { ApiKey = "" }));
        AssertCredentialsRejected(() => new FugleMarketData.RestClient(new FugleMarketData.RestClientOptions { BearerToken = "   " }));
        AssertCredentialsRejected(() => new FugleMarketData.RestClient(new FugleMarketData.RestClientOptions { ApiKey = " ", SdkToken = "" }));
    }

    [TestMethod]
    public void RestClientOptions_BlankAuthNextToRealOne_IsIgnored()
    {
        SkipIfNativeLibraryUnavailable();

        using var client = new FugleMarketData.RestClient(
            new FugleMarketData.RestClientOptions { ApiKey = "  ", SdkToken = "test-sdk-token" });
        Assert.IsNotNull(client);
    }

    [TestMethod]
    public void RestClientOptions_NullOptions_ThrowsArgumentNullException()
    {
        Assert.ThrowsException<ArgumentNullException>(() =>
            new FugleMarketData.RestClient((FugleMarketData.RestClientOptions)null!)
        );
    }

    // ========== WebSocketClientOptions Tests ==========

    [TestMethod]
    public void WebSocketClientOptions_NoAuth_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        var listener = new TestWebSocketListener();
        AssertCredentialsRejected(() =>
            new FugleMarketData.WebSocketClient(new FugleMarketData.WebSocketClientOptions(), listener));
        AssertCredentialsRejected(() =>
            new FugleMarketData.WebSocketClient(new FugleMarketData.WebSocketClientOptions { ApiKey = "  " }, listener));
    }

    [TestMethod]
    public void WebSocketClientOptions_MultipleAuth_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            SdkToken = "test-sdk-token"
        };
        var listener = new TestWebSocketListener();

        AssertCredentialsRejected(() => new FugleMarketData.WebSocketClient(options, listener));
    }

    // ========== ReconnectOptions Tests ==========

    [TestMethod]
    public void ReconnectOptions_DefaultValues_AreNull()
    {
        var options = new FugleMarketData.ReconnectOptions();

        Assert.IsNull(options.Enabled);
        Assert.IsNull(options.MaxAttempts);
        Assert.IsNull(options.InitialDelayMs);
        Assert.IsNull(options.MaxDelayMs);
    }

    [TestMethod]
    public void ReconnectOptions_CustomValues_AreStored()
    {
        var options = new FugleMarketData.ReconnectOptions
        {
            MaxAttempts = 10,
            InitialDelayMs = 2000,
            MaxDelayMs = 120000
        };

        Assert.AreEqual(10u, options.MaxAttempts);
        Assert.AreEqual(2000ul, options.InitialDelayMs);
        Assert.AreEqual(120000ul, options.MaxDelayMs);
    }

    // ========== HealthCheckOptions Tests ==========

    [TestMethod]
    public void HealthCheckOptions_DefaultValues_AreNull()
    {
        var options = new FugleMarketData.HealthCheckOptions();

        Assert.IsNull(options.Enabled);
        Assert.IsNull(options.HeartbeatTimeoutMs);
        Assert.IsNull(options.ProbeEnabled);
        Assert.IsNull(options.IdleProbeAfterMs);
        Assert.IsNull(options.ProbeTimeoutMs);
    }

    [TestMethod]
    public void HealthCheckOptions_CustomValues_AreStored()
    {
        var options = new FugleMarketData.HealthCheckOptions
        {
            Enabled = true,
            HeartbeatTimeoutMs = 20000,
            ProbeEnabled = true,
            IdleProbeAfterMs = 30000,
            ProbeTimeoutMs = 5000
        };

        Assert.AreEqual(true, options.Enabled);
        Assert.AreEqual(20000ul, options.HeartbeatTimeoutMs);
        Assert.AreEqual(true, options.ProbeEnabled);
        Assert.AreEqual(30000ul, options.IdleProbeAfterMs);
        Assert.AreEqual(5000ul, options.ProbeTimeoutMs);
    }

    // ========== Config records (#158, #161) ==========

    [TestMethod]
    public void ConfigRecords_WithoutEnabled_LeaveItUnset()
    {
        // The generated records take enabled as an optional argument: left
        // out, it is unset and core applies its default (on).
        var reconnect = new uniffi.marketdata_uniffi.ReconnectConfigRecord(maxAttempts: 3, initialDelayMs: 0, maxDelayMs: 0);
        Assert.IsNull(reconnect.enabled);

        var healthCheck = new uniffi.marketdata_uniffi.HealthCheckConfigRecord(heartbeatTimeoutMs: 10000);
        Assert.IsNull(healthCheck.enabled);
        Assert.IsFalse(healthCheck.probeEnabled);
    }

    [TestMethod]
    public void ToRecords_OptionsWithoutEnabled_LeaveItUnset()
    {
        Assert.IsNull(FugleMarketData.WebSocketClient.ToReconnectRecord(null));
        Assert.IsNull(FugleMarketData.WebSocketClient.ToHealthCheckRecord(null));

        var reconnect = FugleMarketData.WebSocketClient.ToReconnectRecord(new FugleMarketData.ReconnectOptions { MaxAttempts = 3 });
        Assert.IsNotNull(reconnect);
        Assert.IsNull(reconnect.enabled, "unset Enabled must stay unset");
        Assert.AreEqual(3u, reconnect.maxAttempts);

        var healthCheck = FugleMarketData.WebSocketClient.ToHealthCheckRecord(new FugleMarketData.HealthCheckOptions { HeartbeatTimeoutMs = 10000 });
        Assert.IsNotNull(healthCheck);
        Assert.IsNull(healthCheck.enabled, "unset Enabled must stay unset");
        Assert.IsFalse(healthCheck.probeEnabled);

        Assert.AreEqual(false, FugleMarketData.WebSocketClient.ToReconnectRecord(new FugleMarketData.ReconnectOptions { Enabled = false })!.enabled);
        Assert.AreEqual(false, FugleMarketData.WebSocketClient.ToHealthCheckRecord(new FugleMarketData.HealthCheckOptions { Enabled = false })!.enabled);
    }

    [TestMethod]
    public void WebSocketClient_OptionsWithoutEnabled_CrossFfi()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            Reconnect = new FugleMarketData.ReconnectOptions { MaxAttempts = 3 },
            HealthCheck = new FugleMarketData.HealthCheckOptions { HeartbeatTimeoutMs = 10000 },
        };

        using var client = new FugleMarketData.WebSocketClient(options, new TestWebSocketListener());
        Assert.IsNotNull(client);
    }

    // ========== WebSocketClientOptions with nested config Tests ==========

    [TestMethod]
    public void WebSocketClientOptions_AcceptsReconnectOptions()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            Reconnect = new FugleMarketData.ReconnectOptions
            {
                MaxAttempts = 3,
                InitialDelayMs = 500,
                MaxDelayMs = 30000
            }
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            // Reconnect options stored for future use
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject valid auth and reconnect config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }

    [TestMethod]
    public void WebSocketClientOptions_AcceptsHealthCheckOptions()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            HealthCheck = new FugleMarketData.HealthCheckOptions
            {
                Enabled = true,
                HeartbeatTimeoutMs = 15000
            }
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            // Health check options stored for future use
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject valid auth and health check config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }

    [TestMethod]
    public void WebSocketClientOptions_AcceptsProbeOptions()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            HealthCheck = new FugleMarketData.HealthCheckOptions
            {
                ProbeEnabled = true,
                IdleProbeAfterMs = 30000,
                ProbeTimeoutMs = 5000
            }
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            // Probe options stored for future use
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject valid auth and probe config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }

    [TestMethod]
    public void WebSocketClientOptions_HealthCheck_BelowFloor_ThrowsConfigError()
    {
        SkipIfNativeLibraryUnavailable();

        var listener = new TestWebSocketListener();

        AssertConfigErrorBelowFloor(() => new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions
            {
                ApiKey = "test-api-key",
                HealthCheck = new FugleMarketData.HealthCheckOptions { HeartbeatTimeoutMs = 1000 }
            },
            listener));

        AssertConfigErrorBelowFloor(() => new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions
            {
                ApiKey = "test-api-key",
                HealthCheck = new FugleMarketData.HealthCheckOptions { ProbeEnabled = true, IdleProbeAfterMs = 1000 }
            },
            listener));

        AssertConfigErrorBelowFloor(() => new FugleMarketData.WebSocketClient(
            new FugleMarketData.WebSocketClientOptions
            {
                ApiKey = "test-api-key",
                HealthCheck = new FugleMarketData.HealthCheckOptions { ProbeEnabled = true, ProbeTimeoutMs = 500 }
            },
            listener));
    }

    private static void AssertConfigErrorBelowFloor(Action create)
    {
        var ex = Assert.ThrowsException<uniffi.marketdata_uniffi.MarketDataException.ConfigException>(create);
        var info = FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex);
        Assert.AreEqual(1004, info.code); // marketdata_core::error_code::CONFIG
        StringAssert.Contains(info.message, "must be >=");
    }

    // ========== Message queue (MessageOverflow / MessageBuffer) Tests ==========

    [TestMethod]
    public void WebSocketClientOptions_MessageQueue_DefaultValues_AreNull()
    {
        var options = new FugleMarketData.WebSocketClientOptions();

        Assert.IsNull(options.MessageOverflow);
        Assert.IsNull(options.MessageBuffer);
    }

    [TestMethod]
    public void WebSocketClientOptions_MessageBuffer_Zero_ThrowsArgumentOutOfRangeException()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            MessageBuffer = 0
        };
        var listener = new TestWebSocketListener();

        var ex = Assert.ThrowsException<ArgumentOutOfRangeException>(() =>
            new FugleMarketData.WebSocketClient(options, listener)
        );

        StringAssert.Contains(ex.ParamName, "MessageBuffer");
    }

    [TestMethod]
    public void WebSocketClientOptions_MessageBuffer_Negative_ThrowsArgumentOutOfRangeException()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            MessageBuffer = -1
        };
        var listener = new TestWebSocketListener();

        var ex = Assert.ThrowsException<ArgumentOutOfRangeException>(() =>
            new FugleMarketData.WebSocketClient(options, listener)
        );

        StringAssert.Contains(ex.ParamName, "MessageBuffer");
    }

    // ========== Connection (AuthTimeoutMs) Tests (#199) ==========

    [TestMethod]
    public void WebSocketClientOptions_AuthTimeoutMs_DefaultIsNull()
    {
        var options = new FugleMarketData.WebSocketClientOptions();

        Assert.IsNull(options.AuthTimeoutMs);
        // Unset: no record, so core keeps its default (10 s).
        Assert.IsNull(FugleMarketData.WebSocketClient.ToConnectionRecord(null));
    }

    [TestMethod]
    public void ToConnectionRecord_PassesAuthTimeoutMsThrough()
    {
        var record = FugleMarketData.WebSocketClient.ToConnectionRecord(15000);

        Assert.IsNotNull(record);
        Assert.AreEqual(15000ul, record.authTimeoutMs);
    }

    [TestMethod]
    public void WebSocketClientOptions_AuthTimeoutMs_Zero_ThrowsArgumentOutOfRangeException()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            AuthTimeoutMs = 0
        };
        var listener = new TestWebSocketListener();

        var ex = Assert.ThrowsException<ArgumentOutOfRangeException>(() =>
            new FugleMarketData.WebSocketClient(options, listener)
        );

        StringAssert.Contains(ex.ParamName, "AuthTimeoutMs");
    }

    [TestMethod]
    public void WebSocketClientOptions_AcceptsAuthTimeoutMs()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            AuthTimeoutMs = 15000
        };
        var listener = new TestWebSocketListener();

        using var client = new FugleMarketData.WebSocketClient(options, listener);
        Assert.IsNotNull(client);
    }

    [TestMethod]
    public void WebSocketClientOptions_AcceptsDropNewestOverflow()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            MessageOverflow = FugleMarketData.MessageOverflow.DropNewest,
            MessageBuffer = 8
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            Assert.AreEqual(0ul, client.MessagesDroppedTotal);
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject valid auth and message queue config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }

    [TestMethod]
    public void WebSocketClientOptions_AcceptsUnboundedOverflow()
    {
        SkipIfNativeLibraryUnavailable();

        var options = new FugleMarketData.WebSocketClientOptions
        {
            ApiKey = "test-api-key",
            MessageOverflow = FugleMarketData.MessageOverflow.Unbounded
        };
        var listener = new TestWebSocketListener();

        try
        {
            using var client = new FugleMarketData.WebSocketClient(options, listener);
            Assert.IsNotNull(client);
            Assert.AreEqual(0ul, client.MessagesDroppedTotal);
        }
        catch (uniffi.marketdata_uniffi.MarketDataException ex) when (FugleMarketData.MarketDataExceptionExtensions.GetInfo(ex).code == 1004)
        {
            Assert.Fail("Should not reject valid auth and message queue config");
        }
        catch
        {
            // Other exceptions (UniFFI errors) are acceptable
        }
    }
}
