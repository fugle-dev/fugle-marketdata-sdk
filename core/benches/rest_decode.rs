//! End-to-end REST request cost through `RestClient`, against an in-process
//! HTTP/1.1 keep-alive server.
//!
//! Measures what the SDK adds on top of the network: request building,
//! connection reuse, response decompression and JSON decoding. Real round
//! trips to the Fugle API take tens of milliseconds, so the absolute numbers
//! here matter most for large responses (historical candles) and for
//! concurrent callers sharing one client.
//!
//! ```bash
//! cargo bench -p fugle-marketdata-core --bench rest_decode
//! ```

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use marketdata_core::{Auth, RestClient};

const QUOTE: &[u8] = include_bytes!("fixtures/stock_intraday_quote.json");

fn candles_json(n: usize) -> Vec<u8> {
    let mut s = String::from(
        r#"{"symbol":"2330","type":"EQUITY","exchange":"TWSE","market":"TSE","timeframe":"D","adjusted":false,"data":["#,
    );
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r#"{{"date":"2024-{:02}-{:02}","open":{}.5,"high":{}.0,"low":{}.5,"close":{}.0,"volume":{},"turnover":{},"change":{}.5}}"#,
            i % 12 + 1,
            i % 28 + 1,
            900 + i % 97,
            910 + i % 89,
            890 + i % 83,
            905 + i % 79,
            20_000_000 + i * 131,
            18_000_000_000u64 + i as u64 * 7919,
            i % 7
        ));
    }
    s.push_str("]}");
    s.into_bytes()
}

fn http_response(body: &[u8], gzip: bool) -> Vec<u8> {
    let (payload, encoding) = if gzip {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(body).unwrap();
        (enc.finish().unwrap(), "Content-Encoding: gzip\r\n")
    } else {
        (body.to_vec(), "")
    };
    let mut out = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{encoding}Content-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        payload.len()
    )
    .into_bytes();
    out.extend_from_slice(&payload);
    out
}

/// Starts the server and returns a base URL for `RestClient::base_url`.
/// Symbols pick the response: `2330` quote, `LARGE` candles, `LARGEGZ` gzip.
fn start_server() -> String {
    let candles = candles_json(2000);
    let routes: &'static [(&'static str, Vec<u8>)] = Box::leak(
        vec![
            ("/stock/intraday/quote/", http_response(QUOTE, false)),
            ("/stock/historical/candles/LARGEGZ", http_response(&candles, true)),
            ("/stock/historical/candles/LARGE", http_response(&candles, false)),
        ]
        .into_boxed_slice(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || serve(stream, routes));
        }
    });
    format!("http://127.0.0.1:{port}/marketdata")
}

fn serve(mut stream: TcpStream, routes: &[(&str, Vec<u8>)]) {
    stream.set_nodelay(true).ok();
    let mut buf = vec![0u8; 16 * 1024];
    let mut filled = 0;
    loop {
        match stream.read(&mut buf[filled..]) {
            Ok(0) | Err(_) => return,
            Ok(n) => filled += n,
        }
        while let Some(end) = buf[..filled].windows(4).position(|w| w == b"\r\n\r\n") {
            let head = std::str::from_utf8(&buf[..end]).unwrap_or("");
            let Some((_, body)) = routes.iter().find(|(p, _)| head.contains(p)) else {
                return;
            };
            if stream.write_all(body).is_err() {
                return;
            }
            buf.copy_within(end + 4..filled, 0);
            filled -= end + 4;
        }
    }
}

fn bench_rest_decode(c: &mut Criterion) {
    let base = start_server();
    let client = RestClient::new(Auth::ApiKey("bench".into())).base_url(&base);
    let candles_len = candles_json(2000).len() as u64;

    let mut group = c.benchmark_group("rest_decode");
    group.throughput(Throughput::Bytes(QUOTE.len() as u64));
    group.bench_function("stock_intraday_quote", |b| {
        b.iter(|| black_box(client.stock().intraday().quote().symbol("2330").send().unwrap()))
    });
    group.throughput(Throughput::Bytes(candles_len));
    group.bench_function("historical_candles_2000", |b| {
        b.iter(|| black_box(client.stock().historical().candles().symbol("LARGE").send().unwrap()))
    });
    group.bench_function("historical_candles_2000_gzip", |b| {
        b.iter(|| black_box(client.stock().historical().candles().symbol("LARGEGZ").send().unwrap()))
    });
    group.finish();

    // Eight threads sharing one client, as bindings do when callers issue
    // requests concurrently. Sensitive to the agent's idle-connection pool.
    let mut group = c.benchmark_group("rest_concurrent");
    group.throughput(Throughput::Elements(8 * 50));
    group.bench_function("stock_intraday_quote_8x50", |b| {
        b.iter(|| {
            thread::scope(|s| {
                for _ in 0..8 {
                    let client = client.clone();
                    s.spawn(move || {
                        for _ in 0..50 {
                            black_box(client.stock().intraday().quote().symbol("2330").send().unwrap());
                        }
                    });
                }
            })
        })
    });
    group.finish();
}

criterion_group!(benches, bench_rest_decode);
criterion_main!(benches);
