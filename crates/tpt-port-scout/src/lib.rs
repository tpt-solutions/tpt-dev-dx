//! RAII TCP/UDP port reservation for parallel integration tests.
//!
//! Finding a free port with the classic "bind to `:0`, read the port, drop the
//! listener, then hand the number to your server" pattern has a
//! [TOCTOU](https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use) race:
//! between dropping the probe listener and your server binding, another process
//! (or another parallel test) can grab the same port.
//!
//! `tpt-port-scout` closes that window by **keeping the reservation sockets open**
//! until you are ready to hand them off. A [`PortSet`] binds `N` listeners on
//! `127.0.0.1:0`, exposes each assigned [`SocketAddr`], and only releases the
//! kernel reservations when it is dropped — or when you explicitly convert a
//! listener into a raw handle for your server to reuse.
//!
//! # Quick start
//!
//! ```
//! use tpt_port_scout::PortSet;
//!
//! // Reserve two free ports; both remain held until `ports` is dropped.
//! let ports = PortSet::reserve(2).unwrap();
//! let addr_a = ports.addr(0);
//! let addr_b = ports.addr(1);
//! assert_ne!(addr_a.port(), addr_b.port());
//! ```
//!
//! # Handing a port off to a server
//!
//! There are two supported patterns:
//!
//! 1. **Rebind-before-drop** (portable, works everywhere). Read [`addr`](PortSet::addr),
//!    then drop the [`PortSet`] immediately before your server binds. The race
//!    window is as small as possible, but not zero.
//!
//! 2. **Zero-race hand-off** (recommended). Convert the reservation listener into
//!    a raw OS handle with [`PortSet::into_std_listener`] / [`PortSet::take_listener`]
//!    and let your server adopt the already-bound socket, so the port is *never*
//!    released. See [`PortSet::take_listener`].
//!
//! This crate has **zero runtime dependencies** — pure [`std::net`].

use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket};

/// Loopback address used for all reservations.
const LOOPBACK: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// A set of reserved TCP ports on `127.0.0.1`.
///
/// Each reservation is a live [`TcpListener`] bound to an ephemeral port. The
/// kernel will not reassign those ports to anyone else while the `PortSet` is
/// alive, so parallel tests can each hold their own non-colliding ports.
///
/// Dropping the `PortSet` closes every listener and releases the ports.
pub struct PortSet {
    listeners: Vec<Option<TcpListener>>,
    addrs: Vec<SocketAddr>,
}

impl PortSet {
    /// Reserve `n` free TCP ports on `127.0.0.1`.
    ///
    /// Returns an error if `n` is `0` or if binding any listener fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tpt_port_scout::PortSet;
    /// let ports = PortSet::reserve(3).unwrap();
    /// assert_eq!(ports.len(), 3);
    /// ```
    pub fn reserve(n: usize) -> io::Result<Self> {
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PortSet::reserve requires n >= 1",
            ));
        }
        let mut listeners = Vec::with_capacity(n);
        let mut addrs = Vec::with_capacity(n);
        for _ in 0..n {
            let listener = TcpListener::bind((LOOPBACK, 0))?;
            let addr = listener.local_addr()?;
            listeners.push(Some(listener));
            addrs.push(addr);
        }
        Ok(Self { listeners, addrs })
    }

    /// Reserve a single free TCP port on `127.0.0.1`.
    ///
    /// Convenience wrapper around [`PortSet::reserve(1)`](PortSet::reserve).
    ///
    /// # Examples
    ///
    /// ```
    /// # use tpt_port_scout::PortSet;
    /// let port = PortSet::reserve_one().unwrap();
    /// assert_eq!(port.len(), 1);
    /// let _addr = port.addr(0);
    /// ```
    pub fn reserve_one() -> io::Result<Self> {
        Self::reserve(1)
    }

    /// Number of ports held by this set.
    pub fn len(&self) -> usize {
        self.addrs.len()
    }

    /// Returns `true` if this set holds no (live) reservations.
    ///
    /// A `PortSet` is only empty after all of its listeners have been taken via
    /// [`take_listener`](PortSet::take_listener); it is never empty on construction.
    pub fn is_empty(&self) -> bool {
        self.listeners.iter().all(|l| l.is_none())
    }

    /// The [`SocketAddr`] assigned to reservation `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i` is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tpt_port_scout::PortSet;
    /// let ports = PortSet::reserve_one().unwrap();
    /// let addr = ports.addr(0);
    /// assert!(addr.ip().is_loopback());
    /// assert_ne!(addr.port(), 0);
    /// ```
    pub fn addr(&self, i: usize) -> SocketAddr {
        self.addrs[i]
    }

    /// The [`SocketAddr`] assigned to reservation `i`, or `None` if out of range.
    pub fn try_addr(&self, i: usize) -> Option<SocketAddr> {
        self.addrs.get(i).copied()
    }

    /// The port number assigned to reservation `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i` is out of range.
    pub fn port(&self, i: usize) -> u16 {
        self.addrs[i].port()
    }

    /// All reserved [`SocketAddr`]s, in reservation order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tpt_port_scout::PortSet;
    /// let ports = PortSet::reserve(2).unwrap();
    /// let addrs = ports.addrs();
    /// assert_eq!(addrs.len(), 2);
    /// ```
    pub fn addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }

    /// Borrow the live [`TcpListener`] for reservation `i`.
    ///
    /// Returns `None` if `i` is out of range or the listener has already been
    /// taken via [`take_listener`](PortSet::take_listener).
    pub fn listener(&self, i: usize) -> Option<&TcpListener> {
        self.listeners.get(i).and_then(|l| l.as_ref())
    }

    /// Take ownership of the reservation listener for index `i` for a **zero-race
    /// hand-off** to a server.
    ///
    /// The returned [`TcpListener`] is the *same* already-bound socket that was
    /// holding the reservation, so the port is never released between reservation
    /// and server start-up. Most server frameworks accept a pre-bound
    /// `std::net::TcpListener` (e.g. `hyper`, `axum::Server::from_tcp`,
    /// `actix_web::HttpServer::listen`), which makes this completely race-free.
    ///
    /// After the listener is taken, [`addr`](PortSet::addr) still returns the
    /// address, but the reservation for index `i` is no longer held by this set.
    ///
    /// Returns `None` if `i` is out of range or the listener was already taken.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tpt_port_scout::PortSet;
    /// let mut ports = PortSet::reserve_one().unwrap();
    /// let addr = ports.addr(0);
    /// let listener = ports.take_listener(0).unwrap();
    /// assert_eq!(listener.local_addr().unwrap(), addr);
    /// // hand `listener` to your server here — the port was never released.
    /// ```
    pub fn take_listener(&mut self, i: usize) -> Option<TcpListener> {
        self.listeners.get_mut(i).and_then(|slot| slot.take())
    }

    /// Consume a single-port set and return its underlying [`TcpListener`].
    ///
    /// Convenience for the common `reserve_one()` → hand-off flow.
    ///
    /// # Errors
    ///
    /// Returns an error if the set does not hold exactly one live reservation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tpt_port_scout::PortSet;
    /// let ports = PortSet::reserve_one().unwrap();
    /// let listener = ports.into_std_listener().unwrap();
    /// assert!(listener.local_addr().unwrap().ip().is_loopback());
    /// ```
    pub fn into_std_listener(mut self) -> io::Result<TcpListener> {
        let live: Vec<usize> = self
            .listeners
            .iter()
            .enumerate()
            .filter_map(|(i, l)| l.as_ref().map(|_| i))
            .collect();
        match live.as_slice() {
            [i] => Ok(self.listeners[*i].take().expect("checked as Some above")),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "into_std_listener requires exactly one live reservation, found {}",
                    live.len()
                ),
            )),
        }
    }
}

impl Drop for PortSet {
    fn drop(&mut self) {
        // Explicitly release every remaining reservation. Dropping each
        // `TcpListener` closes its socket and frees the port. Listeners that
        // were taken via `take_listener` / `into_std_listener` are `None` and
        // are owned by the caller instead.
        for slot in self.listeners.iter_mut() {
            drop(slot.take());
        }
    }
}

impl std::fmt::Debug for PortSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PortSet")
            .field("addrs", &self.addrs)
            .field(
                "held",
                &self.listeners.iter().filter(|l| l.is_some()).count(),
            )
            .finish()
    }
}

/// Reserve a set of free UDP ports on `127.0.0.1`.
///
/// UDP counterpart to [`PortSet`]. Each reservation is a bound [`UdpSocket`];
/// the ports are released on drop.
pub struct UdpPortSet {
    sockets: Vec<Option<UdpSocket>>,
    addrs: Vec<SocketAddr>,
}

impl UdpPortSet {
    /// Reserve `n` free UDP ports on `127.0.0.1`.
    ///
    /// Returns an error if `n` is `0` or if binding any socket fails.
    pub fn reserve(n: usize) -> io::Result<Self> {
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "UdpPortSet::reserve requires n >= 1",
            ));
        }
        let mut sockets = Vec::with_capacity(n);
        let mut addrs = Vec::with_capacity(n);
        for _ in 0..n {
            let socket = UdpSocket::bind((LOOPBACK, 0))?;
            let addr = socket.local_addr()?;
            sockets.push(Some(socket));
            addrs.push(addr);
        }
        Ok(Self { sockets, addrs })
    }

    /// Reserve a single free UDP port on `127.0.0.1`.
    pub fn reserve_one() -> io::Result<Self> {
        Self::reserve(1)
    }

    /// Number of ports held by this set.
    pub fn len(&self) -> usize {
        self.addrs.len()
    }

    /// Returns `true` if this set holds no live reservations.
    pub fn is_empty(&self) -> bool {
        self.sockets.iter().all(|s| s.is_none())
    }

    /// The [`SocketAddr`] assigned to reservation `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i` is out of range.
    pub fn addr(&self, i: usize) -> SocketAddr {
        self.addrs[i]
    }

    /// The port number assigned to reservation `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i` is out of range.
    pub fn port(&self, i: usize) -> u16 {
        self.addrs[i].port()
    }

    /// All reserved [`SocketAddr`]s, in reservation order.
    pub fn addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }

    /// Take ownership of the reservation [`UdpSocket`] for index `i` for a
    /// zero-race hand-off.
    pub fn take_socket(&mut self, i: usize) -> Option<UdpSocket> {
        self.sockets.get_mut(i).and_then(|slot| slot.take())
    }
}

impl Drop for UdpPortSet {
    fn drop(&mut self) {
        for slot in self.sockets.iter_mut() {
            drop(slot.take());
        }
    }
}

impl std::fmt::Debug for UdpPortSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpPortSet")
            .field("addrs", &self.addrs)
            .field("held", &self.sockets.iter().filter(|s| s.is_some()).count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::net::TcpStream;

    #[test]
    fn reserve_one_binds_a_loopback_port() {
        let ports = PortSet::reserve_one().unwrap();
        assert_eq!(ports.len(), 1);
        assert!(!ports.is_empty());
        let addr = ports.addr(0);
        assert!(addr.ip().is_loopback());
        assert_ne!(addr.port(), 0);
    }

    #[test]
    fn reserve_zero_is_an_error() {
        assert!(PortSet::reserve(0).is_err());
        assert!(UdpPortSet::reserve(0).is_err());
    }

    #[test]
    fn reserve_many_yields_distinct_ports() {
        let ports = PortSet::reserve(16).unwrap();
        assert_eq!(ports.len(), 16);
        let unique: HashSet<u16> = ports.addrs().iter().map(|a| a.port()).collect();
        assert_eq!(unique.len(), 16, "all reserved ports must be distinct");
    }

    #[test]
    fn parallel_reservations_do_not_collide() {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::new();
        for _ in 0..8 {
            let tx = tx.clone();
            handles.push(thread::spawn(move || {
                // Hold the reservation while we report it, so the kernel can't
                // hand the same port to another thread mid-test.
                let ports = PortSet::reserve(4).unwrap();
                let assigned: Vec<u16> = ports.addrs().iter().map(|a| a.port()).collect();
                tx.send((ports, assigned)).unwrap();
            }));
        }
        drop(tx);
        for h in handles {
            h.join().unwrap();
        }

        // Collect while all guards are still alive → no two ports may match.
        let mut held = Vec::new();
        let mut all_ports = HashSet::new();
        while let Ok((guard, assigned)) = rx.recv() {
            for p in assigned {
                assert!(
                    all_ports.insert(p),
                    "port {p} was reserved twice in parallel"
                );
            }
            held.push(guard);
        }
        assert_eq!(all_ports.len(), 8 * 4);
    }

    #[test]
    fn reserved_port_actually_accepts_connections() {
        // The reservation listener is live, so a client can connect to it.
        let ports = PortSet::reserve_one().unwrap();
        let addr = ports.addr(0);
        let listener = ports.listener(0).unwrap();
        let _client = TcpStream::connect(addr).unwrap();
        let (accepted, _peer) = listener.accept().unwrap();
        assert_eq!(accepted.local_addr().unwrap(), addr);
    }

    #[test]
    fn take_listener_hands_off_same_socket() {
        let mut ports = PortSet::reserve_one().unwrap();
        let addr = ports.addr(0);
        let listener = ports.take_listener(0).unwrap();
        assert_eq!(listener.local_addr().unwrap(), addr);
        // Taken index is now empty; taking again yields None.
        assert!(ports.take_listener(0).is_none());
        assert!(ports.is_empty());
        // The handed-off listener still works.
        let _client = TcpStream::connect(addr).unwrap();
        let (_accepted, _peer) = listener.accept().unwrap();
    }

    #[test]
    fn into_std_listener_requires_exactly_one() {
        let single = PortSet::reserve_one().unwrap();
        assert!(single.into_std_listener().is_ok());

        let many = PortSet::reserve(2).unwrap();
        assert!(many.into_std_listener().is_err());
    }

    #[test]
    fn try_addr_and_out_of_range() {
        let ports = PortSet::reserve(2).unwrap();
        assert!(ports.try_addr(0).is_some());
        assert!(ports.try_addr(2).is_none());
        assert!(ports.listener(5).is_none());
    }

    #[test]
    fn drop_releases_ports_for_reuse() {
        let addr = {
            let ports = PortSet::reserve_one().unwrap();
            ports.addr(0)
        }; // dropped here → port released
           // We should now be able to bind that exact port ourselves.
        let rebound = TcpListener::bind(addr).unwrap();
        assert_eq!(rebound.local_addr().unwrap().port(), addr.port());
    }

    #[test]
    fn udp_reserve_yields_distinct_ports() {
        let ports = UdpPortSet::reserve(8).unwrap();
        assert_eq!(ports.len(), 8);
        let unique: HashSet<u16> = ports.addrs().iter().map(|a| a.port()).collect();
        assert_eq!(unique.len(), 8);
        assert!(ports.addr(0).ip().is_loopback());
    }

    #[test]
    fn udp_take_socket_hands_off() {
        let mut ports = UdpPortSet::reserve_one().unwrap();
        let addr = ports.addr(0);
        let socket = ports.take_socket(0).unwrap();
        assert_eq!(socket.local_addr().unwrap(), addr);
        assert!(ports.is_empty());
    }

    #[test]
    fn debug_impl_reports_held_count() {
        let mut ports = PortSet::reserve(2).unwrap();
        let dbg = format!("{ports:?}");
        assert!(dbg.contains("held: 2"));
        ports.take_listener(0);
        let dbg = format!("{ports:?}");
        assert!(dbg.contains("held: 1"));
    }
}
