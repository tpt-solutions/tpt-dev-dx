# tpt-http-stub

[![crates.io](https://img.shields.io/crates/v/tpt-http-stub.svg)](https://crates.io/crates/tpt-http-stub)
[![docs.rs](https://docs.rs/tpt-http-stub/badge.svg)](https://docs.rs/tpt-http-stub)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

Lightweight **in-process HTTP stub server** for tests.

Spin up a real HTTP server on a random free port (reserved via
[`tpt-port-scout`](https://docs.rs/tpt-port-scout), so there are no TOCTOU
races between parallel tests), register route responses, and assert on the
requests your code actually made. **No async runtime required** — a blocking
thread-per-connection server keeps it simple and synchronous.

## Quick Start

```toml
[dev-dependencies]
tpt-http-stub = "0.1"
```

```rust,ignore
use tpt_http_stub::HttpStub;

let stub = HttpStub::new();
stub.on("GET", "/users/1").respond(200, "alice");

// Point your client at the base URL.
let client = my_http_client(stub.base_url());
assert_eq!(client.get("/users/1"), "alice");

stub.assert_called_once();
```

## Stubbing responses

```rust
use tpt_http_stub::HttpStub;

let stub = HttpStub::new();

// Plain text.
stub.on("GET", "/status").respond(200, "ok");

// JSON — serialised for you.
stub.on("POST", "/items").respond_json(201, &serde_json::json!({"id": 7}));

// Requests to unregistered routes return 500 ("no stub registered …").
```

## Asserting on requests

```rust
use tpt_http_stub::HttpStub;

let stub = HttpStub::new();
stub.on("GET", "/search").respond(200, "ok");
// ... make a request to /search?q=rust ...

let req = stub.last_request().unwrap();
assert_eq!(req.method, "GET");
assert_eq!(req.path_only(), "/search");
assert_eq!(req.query.get("q").as_deref(), Some("rust"));

stub.assert_called_once();
stub.assert_called_n(1);
```

## Parallel tests

Each `HttpStub` binds its own port and runs on its own threads, so concurrent
stubs in parallel tests never interfere.

## License

MIT OR Apache-2.0
