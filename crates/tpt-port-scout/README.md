# tpt-port-scout

[![crates.io](https://img.shields.io/crates/v/tpt-port-scout.svg)](https://crates.io/crates/tpt-port-scout)
[![docs.rs](https://docs.rs/tpt-port-scout/badge.svg)](https://docs.rs/tpt-port-scout)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

RAII TCP/UDP port reservation for parallel integration tests.

The classic "bind to `:0`, read the port, drop the probe, hand the number to the
server" pattern has a **TOCTOU race**: between dropping the probe and the server
binding, another parallel test can steal the same port. `tpt-port-scout` keeps
the reservation socket **open** until you are ready to hand it off, so no two
parallel tests ever collide.

**Zero runtime dependencies** — pure `std::net`.

## Quick Start

```toml
[dev-dependencies]
tpt-port-scout = "0.1"
```

```rust
use tpt_port_scout::PortSet;

#[test]
fn parallel_safe_ports() {
    // Reserve two free ports; both held until `ports` is dropped.
    let ports = PortSet::reserve(2).unwrap();
    let db_addr = ports.addr(0);
    let api_addr = ports.addr(1);
    assert_ne!(db_addr.port(), api_addr.port());
}
```

## Handing a port off to a server

### Zero-race hand-off (recommended)

Convert the reservation listener into a real `std::net::TcpListener` and let your
server adopt the already-bound socket. The port is **never released**, so the
race window is eliminated entirely. Most frameworks accept a pre-bound listener:

```rust
use tpt_port_scout::PortSet;

let ports = PortSet::reserve_one().unwrap();
let addr = ports.addr(0);
let listener = ports.into_std_listener().unwrap();

// axum:  axum::Server::from_tcp(listener)
// hyper: Server::from_tcp(listener)
// actix: HttpServer::new(...).listen(listener)
```

For a multi-port set, take individual listeners by index:

```rust
use tpt_port_scout::PortSet;

let mut ports = PortSet::reserve(2).unwrap();
let api_listener = ports.take_listener(0).unwrap();
let admin_listener = ports.take_listener(1).unwrap();
// index 0 and 1 are now owned by you; the rest stay reserved.
```

### Raw file-descriptor / socket hand-off

If your server API requires a raw handle rather than a `TcpListener`, extract one
from the taken listener using the platform trait:

```rust
# use tpt_port_scout::PortSet;
let listener = PortSet::reserve_one().unwrap().into_std_listener().unwrap();

#[cfg(unix)]
{
    use std::os::unix::io::IntoRawFd;
    let fd = listener.into_raw_fd();
    // pass `fd` to your server, then rebuild with FromRawFd on the far side.
}

#[cfg(windows)]
{
    use std::os::windows::io::IntoRawSocket;
    let sock = listener.into_raw_socket();
}
```

### Rebind-before-drop (portable, minimal window)

If your server can only take a port *number*, read the address and drop the
`PortSet` immediately before binding. The race window is as small as possible,
though not strictly zero — prefer the hand-off approach when you can:

```rust
use tpt_port_scout::PortSet;

let port = {
    let ports = PortSet::reserve_one().unwrap();
    ports.addr(0).port()
}; // reservation released here
// start_server(port);
```

## UDP

`UdpPortSet` provides the same reservation semantics for UDP sockets:

```rust
use tpt_port_scout::UdpPortSet;

let ports = UdpPortSet::reserve(3).unwrap();
let addr = ports.addr(0);
```

## License

MIT OR Apache-2.0
