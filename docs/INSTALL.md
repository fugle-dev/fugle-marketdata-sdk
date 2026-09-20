# Installation Guide

The SDK is published to each language's own package registry. Pre-releases
(`-rc.N`) go to the same registries but are never selected by default, so
existing users of the legacy `fugle-marketdata` (PyPI) and
`@fugle/marketdata` (npm) packages are not upgraded by accident.

| Language | Package | Registry | Pre-release install |
|---|---|---|---|
| Python | `fugle-marketdata` | PyPI | `pip install --pre fugle-marketdata` |
| Node.js | `@fugle/marketdata` | npm | `npm install @fugle/marketdata@next` |
| C# | `Fugle.MarketData` | NuGet | `dotnet add package Fugle.MarketData --prerelease` |
| Go | `github.com/fugle-dev/fugle-marketdata-go` | Go modules | `go get github.com/fugle-dev/fugle-marketdata-go@<version>` |
| C++ | tarball | GitHub Release | see [C++](#c) |
| Rust | `fugle-marketdata` | crates.io | `cargo add fugle-marketdata@<version>` |

Versions follow three independent tracks. Check the
[Releases page](https://github.com/fugle-dev/fugle-marketdata-sdk/releases) for
the numbers that belong together.

| Track | Languages | Example |
|---|---|---|
| Bindings | Python, Node.js | `3.0.0-rc.7` (PyPI spells it `3.0.0rc7`) |
| UniFFI | C#, Go, C++ | `0.2.0-rc.5` |
| Rust crates | Rust | `0.9.0-rc.5` |

Java bindings exist in the repository but are not published yet.

---

## Python

Wheels are built for CPython 3.8+ using the stable ABI (abi3):

| OS | Architectures |
|---|---|
| Linux (glibc 2.17+) | x86_64, aarch64 |
| macOS | x86_64, arm64 |
| Windows | x64 |

```bash
pip install --pre "fugle-marketdata==3.0.0rc7"
```

```python
from fugle_marketdata import RestClient

client = RestClient(api_key="your-api-key")
print(client.stock.intraday.quote("2330"))
```

## Node.js

The main package pulls in one prebuilt native addon for your platform through
`optionalDependencies`:

| OS | Architectures |
|---|---|
| Linux (glibc) | x64, arm64 |
| macOS | x64, arm64 |
| Windows | x64 |

```bash
npm install @fugle/marketdata@next
```

```javascript
const { RestClient } = require('@fugle/marketdata');

const client = new RestClient({ apiKey: 'your-api-key' });
console.log(await client.stock.intraday.quote({ symbol: '2330' }));
```

## C\#

The package targets `netstandard2.0`, `net8.0` and `net10.0`, and bundles native
libraries for `linux-x64`, `linux-arm64`, `osx-arm64`, `osx-x64` and `win-x64`.

```bash
dotnet add package Fugle.MarketData --prerelease
```

```csharp
using FugleMarketData;

using var client = new RestClient(new RestClientOptions { ApiKey = "your-api-key" });
var quote = await client.Stock.Intraday.GetQuoteAsync("2330");
```

## Go

The module ships prebuilt static libraries and links them with cgo, so no
shared library or `LD_LIBRARY_PATH` is needed at runtime.

| GOOS/GOARCH | Toolchain |
|---|---|
| `darwin/arm64`, `darwin/amd64` | Xcode command line tools |
| `linux/amd64`, `linux/arm64` | gcc or clang |
| `windows/amd64` | MinGW-w64 gcc |

```bash
CGO_ENABLED=1 go get github.com/fugle-dev/fugle-marketdata-go@v0.2.0-rc.5
```

```go
import marketdata "github.com/fugle-dev/fugle-marketdata-go"

client, err := marketdata.NewFugleRestClient(marketdata.WithApiKey("your-api-key"))
```

## C++

C++ has no package registry. Each release attaches one tarball per platform
containing the headers and the UniFFI shared library.

```bash
VERSION=0.2.0-rc.5
PLATFORM=osx-arm64  # or linux-x64, linux-arm64, osx-x64, win-x64
TAG=v3.0.0-rc.7     # the bindings release that shipped this UniFFI version

curl -LO "https://github.com/fugle-dev/fugle-marketdata-sdk/releases/download/${TAG}/fugle-marketdata-cpp-${PLATFORM}-${VERSION}.tar.gz"
tar -xzf "fugle-marketdata-cpp-${PLATFORM}-${VERSION}.tar.gz"
SDK_DIR=./fugle-marketdata-cpp-${PLATFORM}-${VERSION}

c++ -std=c++20 -O2 \
    -I"${SDK_DIR}/include" \
    my_app.cpp "${SDK_DIR}/include/marketdata_uniffi.cpp" \
    -L"${SDK_DIR}/lib" -lmarketdata_uniffi \
    -o my_app

# macOS: export DYLD_LIBRARY_PATH="${SDK_DIR}/lib"
# Linux: export LD_LIBRARY_PATH="${SDK_DIR}/lib"
./my_app
```

The C++ API is **sync-only**: the `cpp` UniFFI feature strips async methods
because `uniffi-bindgen-cpp` does not support them. See
`benchmarks/ws/cpp/bench.cpp` for a WebSocket example.

---

## Troubleshooting

### pip installs 2.x instead of 3.x

Pre-releases need `--pre` or an explicit version such as
`fugle-marketdata==3.0.0rc7`.

### npm installs 1.x instead of 3.x

The 3.x pre-releases are on the `next` dist-tag. Use
`npm install @fugle/marketdata@next` or an explicit version.

### Node.js: "Cannot find module '@fugle/marketdata-<platform>'"

npm skipped the optional dependency, usually because of `--no-optional`,
`--omit=optional`, or a lockfile created on another platform. Reinstall
without those flags, or delete `node_modules` and the lockfile and install
again.

### C#: "Unable to load DLL 'marketdata_uniffi'"

The package bundles native libraries under `runtimes/<rid>/native/`. Make sure
you run on one of the supported RIDs; `dotnet --info` shows the current one.

### Go: "cgo: C compiler not found" or linker errors on Windows

The module requires `CGO_ENABLED=1` and a C toolchain. On Windows, install
MinGW-w64 and make sure its `gcc` is on `PATH`.

### C++: "dyld: Library not loaded: libmarketdata_uniffi.dylib"

Set `DYLD_LIBRARY_PATH` (macOS) or `LD_LIBRARY_PATH` (Linux) before running,
or bake an rpath into the executable with `install_name_tool` or `patchelf`.
