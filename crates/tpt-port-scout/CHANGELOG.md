# Changelog

## [0.1.0] — 2026-07-17

### Added

- Initial release.
- `PortSet::reserve(n)` / `PortSet::reserve_one()` — RAII TCP port reservation on `127.0.0.1`.
- `PortSet::addr` / `addrs` / `port` / `try_addr` — inspect reserved addresses.
- `PortSet::take_listener` / `into_std_listener` — zero-race hand-off of the bound `TcpListener`.
- `UdpPortSet` — UDP counterpart with `reserve` / `take_socket`.
- `Drop` releases all remaining reservations.
