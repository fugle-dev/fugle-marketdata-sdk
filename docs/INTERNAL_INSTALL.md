# Internal Installation Guide

> **⚠️ Internal pre-release only.** The artifacts described here are **not**
> published to public package registries (PyPI / npm / NuGet.org / Maven
> Central). They are intended for internal trial and validation before a
> public stable release. **Do not use in production.**

This guide covers installing a pre-release build of the Fugle MarketData SDK
from the **GitHub Release page** (and GitHub Packages for Java). Pre-releases
are tagged `vX.Y.Z-rc.N`, `vX.Y.Z-beta.N`, `vX.Y.Z-alpha.N`, etc. — any
version with a `-` suffix.

All examples below use `0.2.0-rc.1` as a placeholder — substitute the real
version from the Release page.

---

## Release page layout

Each internal pre-release attaches these asset files:

| File pattern | For |
|---|---|
| `fugle_marketdata-*.whl` (multiple) | Python |
| `fugle-marketdata-*.tgz` | Node.js (one fat tarball, all platforms) |
| `MarketdataUniffi.*.nupkg` | C# |
| `fugle-marketdata-go-<platform>-*.tar.gz` | Go (one per platform) |
| `fugle-marketdata-cpp-<platform>-*.tar.gz` | C++ (one per platform) |

Where `<platform>` is one of `linux-x64`, `osx-arm64`, `osx-x64`, `win-x64`.

Java is **not** attached to the Release page — it's published to GitHub
Packages. See the [Java](#java) section.

---

## Python

Pick the wheel matching your OS and CPU architecture from the Release page,
then install directly from the URL:

```bash
VERSION=0.2.0-rc.1
pip install "https://github.com/kevinypfan/fugle-marketdata-sdk/releases/download/v${VERSION}/fugle_marketdata-${VERSION//-/rc}-cp39-abi3-macosx_11_0_arm64.whl"
```

> **Note on filename**: maturin normalizes semver `0.2.0-rc.1` to PEP 440
> `0.2.0rc1` in the wheel filename. Always copy the exact filename from the
> Release page; don't construct it by hand.

Smoke test (canonical example from `py/README.md`):

```python
from marketdata_py import RestClient, MarketDataError

client = RestClient(api_key="your-api-key")

quote = client.stock.intraday.quote("2330")
print(f"TSMC Price: {quote['closePrice']}")
print(f"Change: {quote['change']}")
print(f"Volume: {quote['total']['tradeVolume']}")
```

> The Python import name is `marketdata_py` (the cdylib module name), not
> `fugle_marketdata` (the pip distribution name). `MarketDataError` is the
> primary exception class; `FugleAPIError` is kept as a legacy alias.

---

## Node.js

The internal release produces one **fat tarball** containing `.node` binaries
for all supported platforms. No platform sub-packages, no optionalDependencies
resolution — just a single file install.

```bash
VERSION=0.2.0-rc.1
curl -LO "https://github.com/kevinypfan/fugle-marketdata-sdk/releases/download/v${VERSION}/fugle-marketdata-${VERSION}.tgz"
npm install ./fugle-marketdata-${VERSION}.tgz
```

Smoke test:

```js
const { RestClient } = require('@fugle/marketdata');
const client = new RestClient({ apiKey: 'YOUR_TOKEN' });
client.stock.intraday.quote({ symbol: '2330' }).then(console.log);
```

> **Note**: This fat tarball is ~5× larger than the public platform-specific
> npm package (because it includes every platform's `.node`). Acceptable for
> internal use. The stable public release uses per-platform optional
> dependencies and is much smaller.

---

## C#

Download the `.nupkg` file from the Release page and register a local NuGet
source:

```bash
VERSION=0.2.0-rc.1
mkdir -p ~/.nuget/fugle-internal
curl -L -o ~/.nuget/fugle-internal/MarketdataUniffi.${VERSION}.nupkg \
  "https://github.com/kevinypfan/fugle-marketdata-sdk/releases/download/v${VERSION}/MarketdataUniffi.${VERSION}.nupkg"

dotnet nuget add source ~/.nuget/fugle-internal -n fugle-internal
```

In your project:

```bash
dotnet add package MarketdataUniffi --version ${VERSION}
```

Smoke test (C#):

```csharp
using FugleMarketData;
var client = new RestClient(new RestClientConfig { ApiKey = "YOUR_TOKEN" });
var quote = await client.Stock.Intraday.QuoteAsync("2330");
Console.WriteLine(quote);
```

---

## Java

Java **is** published to GitHub Packages for both stable and pre-release
versions (this is how Java is distributed in normal production too). You need
a GitHub Personal Access Token (PAT) with the `read:packages` scope.

### One-time PAT setup

1. GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Generate new token, check `read:packages`
3. Export as env var:
   ```bash
   export GITHUB_ACTOR=your-github-username
   export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
   ```

### Gradle

```kotlin
// build.gradle.kts
repositories {
    maven {
        url = uri("https://maven.pkg.github.com/kevinypfan/fugle-marketdata-sdk")
        credentials {
            username = System.getenv("GITHUB_ACTOR")
            password = System.getenv("GITHUB_TOKEN")
        }
    }
}

dependencies {
    implementation("tw.com.fugle:marketdata-java:0.2.0-rc.1")
}
```

### Maven

```xml
<!-- ~/.m2/settings.xml -->
<servers>
  <server>
    <id>github</id>
    <username>${env.GITHUB_ACTOR}</username>
    <password>${env.GITHUB_TOKEN}</password>
  </server>
</servers>

<!-- pom.xml -->
<repositories>
  <repository>
    <id>github</id>
    <url>https://maven.pkg.github.com/kevinypfan/fugle-marketdata-sdk</url>
  </repository>
</repositories>

<dependencies>
  <dependency>
    <groupId>tw.com.fugle</groupId>
    <artifactId>marketdata-java</artifactId>
    <version>0.2.0-rc.1</version>
  </dependency>
</dependencies>
```

---

## Go

Go does not have a package registry in the conventional sense, and the module
path `github.com/fugle-dev/fugle-marketdata-go` is intentionally **not yet a
real repo**. For internal trial, we ship a tarball containing the Go source +
native library, and you override module resolution with a `replace` directive.

```bash
VERSION=0.2.0-rc.1
# Pick the right platform: linux-x64, osx-arm64, osx-x64, win-x64
PLATFORM=osx-arm64

curl -LO "https://github.com/kevinypfan/fugle-marketdata-sdk/releases/download/v${VERSION}/fugle-marketdata-go-${PLATFORM}-${VERSION}.tar.gz"
mkdir -p vendor
tar -xzf "fugle-marketdata-go-${PLATFORM}-${VERSION}.tar.gz" -C vendor/
# This creates vendor/fugle-marketdata-go-<platform>-<version>/{src,lib,README.md}
mv "vendor/fugle-marketdata-go-${PLATFORM}-${VERSION}" vendor/fugle-marketdata-go
```

Add to your `go.mod`:

```go
require github.com/fugle-dev/fugle-marketdata-go v0.0.0-internal

replace github.com/fugle-dev/fugle-marketdata-go => ./vendor/fugle-marketdata-go/src
```

Set CGO flags before building:

```bash
# macOS
export CGO_LDFLAGS="-L$(pwd)/vendor/fugle-marketdata-go/lib -lmarketdata_uniffi"
export DYLD_LIBRARY_PATH="$(pwd)/vendor/fugle-marketdata-go/lib:$DYLD_LIBRARY_PATH"

# Linux
export CGO_LDFLAGS="-L$(pwd)/vendor/fugle-marketdata-go/lib -lmarketdata_uniffi"
export LD_LIBRARY_PATH="$(pwd)/vendor/fugle-marketdata-go/lib:$LD_LIBRARY_PATH"

# Windows (PowerShell)
$env:CGO_LDFLAGS = "-L$(pwd)\vendor\fugle-marketdata-go\lib -lmarketdata_uniffi"
$env:PATH = "$(pwd)\vendor\fugle-marketdata-go\lib;$env:PATH"
```

Smoke test:

```go
package main

import (
    "fmt"
    marketdata "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
    client, err := marketdata.NewRestClient(marketdata.RestClientConfig{
        ApiKey: "YOUR_TOKEN",
    })
    if err != nil { panic(err) }
    quote, err := client.Stock().Intraday().Quote("2330")
    if err != nil { panic(err) }
    fmt.Println(quote)
}
```

---

## C++

C++ has no package manager integration — the tarball ships headers + the
UniFFI native library, and you compile against them directly.

```bash
VERSION=0.2.0-rc.1
PLATFORM=osx-arm64  # or linux-x64, osx-x64, win-x64

curl -LO "https://github.com/kevinypfan/fugle-marketdata-sdk/releases/download/v${VERSION}/fugle-marketdata-cpp-${PLATFORM}-${VERSION}.tar.gz"
tar -xzf "fugle-marketdata-cpp-${PLATFORM}-${VERSION}.tar.gz"
# Creates fugle-marketdata-cpp-<platform>-<version>/{include,lib,README.md}
```

Compile your program:

```bash
SDK_DIR=./fugle-marketdata-cpp-${PLATFORM}-${VERSION}

c++ -std=c++20 -O2 \
    -I"${SDK_DIR}/include" \
    my_app.cpp "${SDK_DIR}/include/marketdata_uniffi.cpp" \
    -L"${SDK_DIR}/lib" -lmarketdata_uniffi \
    -o my_app
```

Runtime:

```bash
# macOS
export DYLD_LIBRARY_PATH="${SDK_DIR}/lib:$DYLD_LIBRARY_PATH"
./my_app

# Linux
export LD_LIBRARY_PATH="${SDK_DIR}/lib:$LD_LIBRARY_PATH"
./my_app
```

The C++ API is **sync-only** (the `cpp` UniFFI feature flag strips async
methods because `uniffi-bindgen-cpp` doesn't support them). For async use
cases, pick a different language binding.

See `benchmarks/ws/cpp/bench.cpp` for a working example using the WebSocket
client with the sync API.

---

## Troubleshooting

### Python: "wheel is not a supported wheel on this platform"

The wheel filename encodes the target platform (e.g. `macosx_11_0_arm64`,
`manylinux_2_17_x86_64`). Download the one matching your actual host.

### Node.js: "Cannot find module '...marketdata-js.xxx.node'"

The fat tarball should contain `.node` files for all platforms. If you get
this error, re-download the tarball (it may have been truncated) and verify
it has all binaries:
```bash
tar -tzf fugle-marketdata-*.tgz | grep .node
```

### C#: "Unable to load DLL 'marketdata_uniffi'"

The `.nupkg` bundles native libraries under `runtimes/<rid>/native/`. Make
sure your project's `TargetFramework` is `net6.0` or later and that you're
running on a supported RID. On macOS ARM64, check `dotnet --info` shows
`osx-arm64`.

### Java: "401 Unauthorized" from GitHub Packages

Your PAT doesn't have `read:packages` scope, or the `GITHUB_TOKEN` env var
isn't exported in the shell running Gradle / Maven. Regenerate with the
correct scope and re-export.

### Go: "undefined reference to uniffi_*" at link time

`CGO_LDFLAGS` isn't set, or the library path is wrong. Double-check that
`./vendor/fugle-marketdata-go/lib/libmarketdata_uniffi.*` exists and the
`CGO_LDFLAGS` points at that directory.

### C++: "dyld: Library not loaded: libmarketdata_uniffi.dylib"

`DYLD_LIBRARY_PATH` (macOS) / `LD_LIBRARY_PATH` (Linux) isn't set in the shell
where you run the binary. Set it before `./my_app`, or use
`install_name_tool` / `patchelf` to bake an rpath into the executable.
