# fugle-marketdata-go

Go SDK for the [Fugle Market Data API](https://developer.fugle.tw), built on the
Rust core of [fugle-marketdata-sdk](https://github.com/fugle-dev/fugle-marketdata-sdk)
via UniFFI.

> **This repository is generated.** Every release is assembled by CI from
> `bindings/go/` in fugle-dev/fugle-marketdata-sdk. Please open issues and pull
> requests there, not here.

Version: `v@VERSION@`

## Installation

```bash
go get github.com/fugle-dev/fugle-marketdata-go@v@VERSION@
```

The module ships a prebuilt static library for each supported platform, so no
Rust toolchain and no `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH` are needed. cgo
must be enabled and a C toolchain must be available for the final link.

| Platform | GOOS/GOARCH | Minimum OS | C toolchain |
|----------|-------------|------------|-------------|
| macOS Apple Silicon | `darwin/arm64` | macOS 11 | Xcode Command Line Tools |
| macOS Intel | `darwin/amd64` | macOS 11 | Xcode Command Line Tools |
| Linux x86-64 (glibc) | `linux/amd64` | glibc 2.17 | gcc or clang |
| Windows x86-64 | `windows/amd64` | Windows 10 | [mingw-w64](https://www.mingw-w64.org/) gcc on `PATH` |

```bash
export CGO_ENABLED=1
```

Not supported in this release: `linux/arm64`, musl-based Linux (Alpine), and
Windows builds using MSVC. `go mod vendor` is not supported because it does not
copy the `lib/` directories.

## Quick start

```go
package main

import (
    "fmt"
    "log"

    mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
    client, err := mkt.NewRestClientWithApiKey("your-api-key")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Destroy()

    quote, err := client.Stock().Intraday().GetQuote("2330")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("%+v\n", quote)
}
```

Full API documentation and WebSocket examples:
<https://github.com/fugle-dev/fugle-marketdata-sdk/tree/main/bindings/go>

## Versioning

The Go SDK follows its own `0.x` version track, shared with the C# binding. Minor
versions may contain breaking changes until `1.0.0`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
