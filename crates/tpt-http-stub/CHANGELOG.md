# Changelog

## [0.1.0] — 2026-07-17

### Added

- Initial release.
- `HttpStub::new()` — blocks a random free port via `tpt-port-scout`.
- `HttpStub::on(method, path).respond(status, body)` and `.respond_json(...)` stub registration.
- `HttpStub::base_url()` — the address clients should target.
- Request capture: `last_request()`, `requests()`, `query` parsing.
- `assert_called_once()`, `assert_called_n(n)`, `assert_called()`.
- `Drop` shuts down the server threads.
- Missing/registered-route returns 500 ("no stub registered").
