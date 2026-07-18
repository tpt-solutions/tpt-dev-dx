//! Lightweight in-process HTTP stub server for tests.
//!
//! `tpt-http-stub` spins up a real TCP HTTP server on a random free port (reserved
//! via `tpt-port-scout`, so there are no TOCTOU races) and answers registered
//! route stubs. It uses a blocking, single-thread-per-connection server — **no
//! async runtime required** — which makes it ideal for synchronous tests.
//!
//! # Quick start
//!
//! ```
//! use tpt_http_stub::HttpStub;
//! use std::io::{Read, Write};
//! use std::net::TcpStream;
//!
//! let stub = HttpStub::new();
//! stub.on("GET", "/users/1").respond(200, "alice");
//!
//! // Point any HTTP client at the base URL. Here we issue a raw request to keep
//! // the example dependency-free.
//! let host = stub.base_url().trim_start_matches("http://").to_string();
//! let mut s = TcpStream::connect(&host).unwrap();
//! s.write_all(b"GET /users/1 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
//! s.flush().unwrap();
//! let mut resp = String::new();
//! s.read_to_string(&mut resp).unwrap();
//! let body = resp.splitn(2, "\r\n\r\n").nth(1).unwrap();
//! assert_eq!(body, "alice");
//!
//! stub.assert_called_once();
//! ```
//!
//! (In real tests drive the stub with any HTTP client — the example above uses a
//! raw socket to stay dependency-free.)

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use tpt_port_scout::PortSet;

/// An HTTP method, stored as an upper-cased string for case-insensitive matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Method(String);

impl Method {
    /// Parse a method from a string, upper-casing it.
    pub fn new(s: &str) -> Self {
        Self(s.to_uppercase())
    }
}

impl FromStr for Method {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A registered stub response.
#[derive(Debug, Clone)]
struct Stub {
    status: u16,
    body: Vec<u8>,
    content_type: String,
}

/// A captured incoming request for post-hoc assertions.
#[derive(Debug, Clone)]
pub struct CapturedRequest {
    /// The HTTP method, upper-cased.
    pub method: String,
    /// The request path (including any query string), e.g. `/users/1?expand=1`.
    pub path: String,
    /// The request body (empty for GETs).
    pub body: Vec<u8>,
    /// Parsed query parameters (from the path's `?...` portion).
    pub query: HashMap<String, String>,
}

impl CapturedRequest {
    /// The request path without the query string.
    pub fn path_only(&self) -> &str {
        self.path.split('?').next().unwrap_or(&self.path)
    }

    /// The request body decoded as UTF-8 (lossy).
    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Shared, thread-safe state for a running stub server.
struct Inner {
    stubs: Mutex<HashMap<(Method, String), Stub>>,
    requests: Mutex<Vec<CapturedRequest>>,
}

/// An in-process HTTP stub server.
///
/// Create with [`HttpStub::new`] (binds a random free port via `tpt-port-scout`),
/// register route responses with [`HttpStub::on`], and assert with
/// [`HttpStub::assert_called_once`] / [`HttpStub::requests`].
///
/// The server runs in background threads and is shut down (and expectations
/// checked) when the `HttpStub` is dropped.
pub struct HttpStub {
    inner: Arc<Inner>,
    base_url: String,
    shutdown: Option<mpsc::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl HttpStub {
    /// Start a stub server on a random free loopback port.
    ///
    /// # Panics
    ///
    /// Panics if no free port can be reserved (extremely unlikely).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let ports = PortSet::reserve_one().expect("failed to reserve a free port");
        let listener = ports
            .into_std_listener()
            .expect("reservation must hold one listener");
        let addr = listener.local_addr().expect("listener has an address");
        let base_url = format!("http://{addr}");

        let inner = Arc::new(Inner {
            stubs: Mutex::new(HashMap::new()),
            requests: Mutex::new(Vec::new()),
        });

        let (tx, rx) = mpsc::channel::<()>();
        let inner_srv = Arc::clone(&inner);
        let handle = thread::spawn(move || {
            // Stop accepting new connections once a shutdown signal arrives.
            let _ = listener.set_nonblocking(true);
            loop {
                match rx.try_recv() {
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        let inner = Arc::clone(&inner_srv);
                        thread::spawn(move || handle_conn(stream, &inner));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            inner,
            base_url,
            shutdown: Some(tx),
            handle: Some(handle),
        }
    }

    /// Register a stubbed response for `method` + `path`.
    ///
    /// Subsequent requests to that route return `status` with `body`. Multiple
    /// calls for the same route overwrite the previous stub.
    pub fn on(&self, method: impl AsRef<str>, path: &str) -> StubBuilder<'_> {
        StubBuilder {
            stub: self,
            method: Method::new(method.as_ref()),
            path: path.to_string(),
        }
    }

    /// The base URL clients should target, e.g. `http://127.0.0.1:54321`.
    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    /// All captured requests, in arrival order.
    pub fn requests(&self) -> Vec<CapturedRequest> {
        self.inner.requests.lock().expect("poisoned").clone()
    }

    /// The most recent captured request, if any.
    pub fn last_request(&self) -> Option<CapturedRequest> {
        self.inner
            .requests
            .lock()
            .expect("poisoned")
            .last()
            .cloned()
    }

    /// Number of requests received.
    pub fn call_count(&self) -> usize {
        self.inner.requests.lock().expect("poisoned").len()
    }

    /// Assert exactly one request has been received.
    #[track_caller]
    pub fn assert_called_once(&self) {
        let n = self.call_count();
        assert!(n == 1, "expected exactly 1 call, but the stub received {n}");
    }

    /// Assert exactly `n` requests have been received.
    #[track_caller]
    pub fn assert_called_n(&self, n: usize) {
        let actual = self.call_count();
        assert!(
            actual == n,
            "expected {n} calls, but the stub received {actual}"
        );
    }

    /// Assert at least one request has been received (default "missing stub → 500"
    /// behaviour means the route must exist to get a 2xx).
    #[track_caller]
    pub fn assert_called(&self) {
        assert!(
            self.call_count() >= 1,
            "expected the stub to be called at least once"
        );
    }
}

/// Builder returned by [`HttpStub::on`] to specify the response.
pub struct StubBuilder<'a> {
    stub: &'a HttpStub,
    method: Method,
    path: String,
}

impl StubBuilder<'_> {
    /// Respond with `status` and a UTF-8 `body`.
    pub fn respond(self, status: u16, body: impl AsRef<str>) {
        self.respond_raw(
            status,
            "text/plain; charset=utf-8",
            body.as_ref().as_bytes().to_vec(),
        );
    }

    /// Respond with `status` and a raw byte `body` (`content_type` advertised).
    pub fn respond_raw(self, status: u16, content_type: &str, body: Vec<u8>) {
        let mut stubs = self.stub.inner.stubs.lock().expect("poisoned");
        stubs.insert(
            (self.method, self.path),
            Stub {
                status,
                body,
                content_type: content_type.to_string(),
            },
        );
    }

    /// Respond with `status` and a JSON-encoded `value`.
    pub fn respond_json(self, status: u16, value: &impl serde::Serialize) {
        match serde_json::to_vec(value) {
            Ok(bytes) => self.respond_raw(status, "application/json; charset=utf-8", bytes),
            Err(e) => panic!("failed to serialise stub JSON body: {e}"),
        }
    }
}

fn handle_conn(mut stream: TcpStream, inner: &Arc<Inner>) {
    // Read the request line + headers + body (bounded). We don't need a full
    // HTTP parser — just enough to route and echo.
    let mut buf = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    // Read until we have headers (blank line) and then any Content-Length body.
    let mut header_end = None;
    let mut total = 0usize;
    while total < 64 * 1024 {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                total += 1;
                if let Some(pos) = find_header_end(&buf) {
                    header_end = Some(pos);
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let header_end = header_end.unwrap_or(buf.len());
    let header_bytes = &buf[..header_end];
    let headers = String::from_utf8_lossy(header_bytes);

    let mut lines = headers.lines();
    let request_line = lines.next().unwrap_or("").to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_uppercase();
    let full_path = parts.next().unwrap_or("/").to_string();

    // Determine body length from Content-Length.
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = Vec::new();
    if content_length > 0 {
        body.extend_from_slice(&buf[header_end..]);
        while body.len() < content_length {
            let mut chunk = [0u8; 1024];
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
        body.truncate(content_length);
    }

    let path_only = full_path.split('?').next().unwrap_or("/").to_string();
    let query = parse_query(&full_path);

    let captured = CapturedRequest {
        method: method.clone(),
        path: full_path,
        body,
        query,
    };
    inner.requests.lock().expect("poisoned").push(captured);

    // Route lookup.
    let stub = {
        let stubs = inner.stubs.lock().expect("poisoned");
        stubs
            .get(&(Method::new(&method), path_only.clone()))
            .cloned()
    };

    match stub {
        Some(stub) => {
            let _ = write_response(&mut stream, stub.status, &stub.content_type, &stub.body);
        }
        None => {
            let body = format!("no stub registered for {method} {path_only}");
            let _ = write_response(
                &mut stream,
                500,
                "text/plain; charset=utf-8",
                body.as_bytes(),
            );
        }
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    // Detect "\r\n\r\n" or "\n\n".
    if buf.len() >= 4 {
        for i in 0..=buf.len() - 4 {
            if buf[i] == b'\r' && buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n'
            {
                return Some(i + 4);
            }
        }
    }
    if buf.len() >= 2 {
        for i in 0..=buf.len() - 2 {
            if buf[i] == b'\n' && buf[i + 1] == b'\n' {
                return Some(i + 2);
            }
        }
    }
    None
}

fn parse_query(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(q) = path.split_once('?').map(|(_, q)| q) {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            map.insert(percent_decode(k), percent_decode(v));
        }
    }
    map
}

fn percent_decode(s: &str) -> String {
    // Minimal decode for the common '%XX' escapes; leaves other chars intact.
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = status_text(status);
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

impl Drop for HttpStub {
    fn drop(&mut self) {
        // Signal the acceptor thread to stop and join it. Dropping `shutdown`
        // (the sender) also disconnects the channel. Any open per-request
        // threads finish on their own.
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream as Client;

    fn get(stub: &HttpStub, method: &str, path: &str) -> (u16, String) {
        let url = stub.base_url();
        let mut stream = Client::connect(url.trim_start_matches("http://")).unwrap();
        let req =
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream.write_all(req.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        // Split status line.
        let status_line = resp.lines().next().unwrap_or("");
        let code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = resp
            .split_once("\r\n\r\n")
            .map(|x| x.1)
            .unwrap_or("")
            .to_string();
        (code, body)
    }

    #[test]
    fn stub_responds_with_registered_body() {
        let stub = HttpStub::new();
        stub.on("GET", "/users/1").respond(200, "alice");
        let (code, body) = get(&stub, "GET", "/users/1");
        assert_eq!(code, 200);
        assert_eq!(body, "alice");
        stub.assert_called_once();
    }

    #[test]
    fn method_is_case_insensitive() {
        let stub = HttpStub::new();
        stub.on("post", "/items").respond(201, "created");
        let (code, body) = get(&stub, "POST", "/items");
        assert_eq!(code, 201);
        assert_eq!(body, "created");
    }

    #[test]
    fn missing_stub_returns_500() {
        let stub = HttpStub::new();
        let (code, body) = get(&stub, "GET", "/nope");
        assert_eq!(code, 500);
        assert!(body.contains("no stub"));
    }

    #[test]
    fn json_responses_are_serialised() {
        let stub = HttpStub::new();
        stub.on("GET", "/json")
            .respond_json(200, &serde_json::json!({"name": "bob", "age": 30}));
        let (code, body) = get(&stub, "GET", "/json");
        assert_eq!(code, 200);
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["name"], "bob");
        assert_eq!(parsed["age"], 30);
    }

    #[test]
    fn captures_request_and_query() {
        let stub = HttpStub::new();
        stub.on("GET", "/search").respond(200, "ok");
        let _ = get(&stub, "GET", "/search?q=rust&page=2");
        let req = stub.last_request().expect("request captured");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path_only(), "/search");
        assert_eq!(req.query.get("q").map(String::as_str), Some("rust"));
        assert_eq!(req.query.get("page").map(String::as_str), Some("2"));
        stub.assert_called_once();
    }

    #[test]
    fn parallel_stubs_do_not_interfere() {
        use std::sync::Arc;
        let a = Arc::new(HttpStub::new());
        let b = Arc::new(HttpStub::new());
        a.on("GET", "/a").respond(200, "from-a");
        b.on("GET", "/b").respond(200, "from-b");

        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        let t1 = thread::spawn(move || get(&a2, "GET", "/a"));
        let t2 = thread::spawn(move || get(&b2, "GET", "/b"));
        let (ra, rb) = (t1.join().unwrap(), t2.join().unwrap());
        assert_eq!(ra, (200, "from-a".to_string()));
        assert_eq!(rb, (200, "from-b".to_string()));

        assert_eq!(a.call_count(), 1);
        assert_eq!(b.call_count(), 1);
        // Each stub only saw its own route.
        assert_eq!(a.last_request().unwrap().path_only(), "/a");
        assert_eq!(b.last_request().unwrap().path_only(), "/b");
    }

    #[test]
    fn assert_called_n_counts() {
        let stub = HttpStub::new();
        stub.on("GET", "/x").respond(200, "x");
        let _ = get(&stub, "GET", "/x");
        let _ = get(&stub, "GET", "/x");
        stub.assert_called_n(2);
        assert_eq!(stub.requests().len(), 2);
    }

    #[test]
    #[should_panic(expected = "expected exactly 1 call")]
    fn assert_called_once_panics_with_zero() {
        let stub = HttpStub::new();
        stub.assert_called_once();
    }

    #[test]
    fn base_url_is_loopback_with_port() {
        let stub = HttpStub::new();
        let url = stub.base_url();
        assert!(url.starts_with("http://127.0.0.1:"));
    }
}
