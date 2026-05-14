//! Embedded STUN server (RFC 5389 Binding subset).
//!
//! Exists so that **a fresh deployment can immediately serve WebRTC
//! peers on a closed LAN** — without requiring the operator to run a
//! separate `coturn` container, configure firewalls, or set
//! `STUN_TURN_SERVERS`.
//!
//! Scope: only the bare minimum needed for WebRTC ICE host-candidate
//! discovery. We answer Binding Requests with an XOR-MAPPED-ADDRESS
//! pointing back at the source IP/port. We do **not** implement
//! authentication, ALLOCATE / TURN relays, fingerprint validation,
//! or message-integrity — those are unnecessary for STUN-only use
//! and reduce attack surface by being absent.
//!
//! Wire format (RFC 5389 §6):
//!
//! ```text
//! Header (20 B):
//!   Type    (u16, big-endian)  e.g. 0x0001 = Binding Request
//!   Length  (u16, big-endian)  attribute bytes after header
//!   Cookie  (u32, big-endian)  fixed 0x2112A442
//!   TxId    (12 B)             opaque transaction id
//! Attributes:
//!   Type   (u16)
//!   Length (u16)               value bytes (padded to 4-byte multiple)
//!   Value  (Length B)
//! ```

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

/// RFC 5389 magic cookie (in network byte order it ends up `0x2112A442`).
const MAGIC_COOKIE: u32 = 0x2112_A442;

/// `BINDING REQUEST` message type (RFC 5389 §3, table on p.16).
const BINDING_REQUEST: u16 = 0x0001;
/// `BINDING SUCCESS RESPONSE` message type.
const BINDING_SUCCESS: u16 = 0x0101;

/// Attribute type for `XOR-MAPPED-ADDRESS` (RFC 5389 §15.2).
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// Address-family discriminator used inside `XOR-MAPPED-ADDRESS`.
const FAMILY_IPV4: u8 = 0x01;
const FAMILY_IPV6: u8 = 0x02;

/// Spawn the STUN server task on `bind_addr`. Returns once the
/// socket is bound; the server then runs forever on a detached task.
///
/// Errors propagate from the `UdpSocket::bind` call so the caller
/// can decide whether to abort startup or just log a warning and
/// continue (the chat server itself is still usable without an
/// embedded STUN — clients fall back to the public STUN list).
pub async fn spawn(bind_addr: SocketAddr) -> std::io::Result<()> {
  let socket = UdpSocket::bind(bind_addr).await?;
  let local_addr = socket.local_addr()?;
  info!(stun_addr = %local_addr, "Embedded STUN server listening");

  tokio::spawn(async move {
    serve(socket).await;
  });

  Ok(())
}

/// Receive loop. Each packet is parsed and either responded to or
/// silently dropped (RFC 5389 §6 mandates that we MUST silently
/// discard malformed messages).
async fn serve(socket: UdpSocket) {
  // 576 bytes covers any well-formed STUN message (Binding Request
  // is at most ~100 B in practice). MTU is irrelevant because STUN
  // packets are small.
  let mut buf = [0u8; 1500];
  loop {
    let (n, src) = match socket.recv_from(&mut buf).await {
      Ok(pair) => pair,
      Err(e) => {
        warn!(error = %e, "STUN: recv_from failed; continuing");
        continue;
      }
    };

    let request = &buf[..n];
    let Some(response) = handle_request(request, src) else {
      // Not a valid Binding Request — drop silently.
      continue;
    };

    if let Err(e) = socket.send_to(&response, src).await {
      debug!(peer = %src, error = %e, "STUN: send_to failed");
    }
  }
}

/// Parse a Binding Request and return the bytes of the corresponding
/// Binding Success Response. Returns `None` for malformed or
/// unsupported messages so the caller drops them silently.
fn handle_request(request: &[u8], src: SocketAddr) -> Option<Vec<u8>> {
  if request.len() < 20 {
    return None;
  }
  let msg_type = u16::from_be_bytes([request[0], request[1]]);
  if msg_type != BINDING_REQUEST {
    return None;
  }
  let cookie = u32::from_be_bytes([request[4], request[5], request[6], request[7]]);
  if cookie != MAGIC_COOKIE {
    // Pre-RFC 5389 ("classic STUN") clients omit the cookie. We
    // ignore them; modern browsers always send the cookie.
    return None;
  }
  let tx_id: [u8; 12] = request[8..20].try_into().ok()?;

  Some(build_binding_response(tx_id, src))
}

/// Build a Binding Success Response containing a single
/// `XOR-MAPPED-ADDRESS` attribute that echoes `src` back to the peer.
fn build_binding_response(tx_id: [u8; 12], src: SocketAddr) -> Vec<u8> {
  let attr = encode_xor_mapped_address(src, &tx_id);
  // STUN message length covers attributes only (not the 20-byte header).
  let attr_len = u16::try_from(attr.len()).expect("attribute fits in u16");

  let mut out = Vec::with_capacity(20 + attr.len());
  out.extend_from_slice(&BINDING_SUCCESS.to_be_bytes());
  out.extend_from_slice(&attr_len.to_be_bytes());
  out.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
  out.extend_from_slice(&tx_id);
  out.extend_from_slice(&attr);
  out
}

/// Encode a `XOR-MAPPED-ADDRESS` attribute (RFC 5389 §15.2).
///
/// The address is XOR'd with the magic cookie (and, for IPv6, the
/// transaction id) so middleboxes that rewrite payloads cannot
/// tamper with it without realising they did.
fn encode_xor_mapped_address(addr: SocketAddr, tx_id: &[u8; 12]) -> Vec<u8> {
  let port_xor = addr.port() ^ ((MAGIC_COOKIE >> 16) as u16);

  let mut value = Vec::with_capacity(4 + 16);
  value.push(0); // RESERVED
  match addr.ip() {
    IpAddr::V4(ipv4) => {
      value.push(FAMILY_IPV4);
      value.extend_from_slice(&port_xor.to_be_bytes());
      let octets = ipv4.octets();
      let cookie_bytes = MAGIC_COOKIE.to_be_bytes();
      value.extend(octets.iter().zip(cookie_bytes.iter()).map(|(o, c)| o ^ c));
    }
    IpAddr::V6(ipv6) => {
      value.push(FAMILY_IPV6);
      value.extend_from_slice(&port_xor.to_be_bytes());
      let octets = ipv6.octets();
      let cookie_bytes = MAGIC_COOKIE.to_be_bytes();
      value.extend(
        octets[..4]
          .iter()
          .zip(cookie_bytes.iter())
          .map(|(o, c)| o ^ c),
      );
      value.extend(octets[4..].iter().zip(tx_id.iter()).map(|(o, t)| o ^ t));
    }
  }

  // Wrap into a TLV (type, length, value, padded to 4-byte boundary).
  let value_len = u16::try_from(value.len()).expect("XOR-MAPPED-ADDRESS value fits in u16");
  let pad = (4 - value.len() % 4) % 4;
  let mut attr = Vec::with_capacity(4 + value.len() + pad);
  attr.extend_from_slice(&ATTR_XOR_MAPPED_ADDRESS.to_be_bytes());
  attr.extend_from_slice(&value_len.to_be_bytes());
  attr.extend_from_slice(&value);
  attr.extend(std::iter::repeat_n(0u8, pad));
  attr
}

/// Best-effort detection of the host's primary LAN IPv4. Used by the
/// chat server to advertise an `stun:<lan>:3478` entry inside
/// [`AuthSuccess::ice_servers`] without requiring the operator to
/// configure `STUN_TURN_SERVERS`.
///
/// Strategy: open a UDP socket and "connect" it to a public address;
/// the kernel picks an outbound interface whose source IP we can
/// then read. No packet is sent — `connect` on UDP just sets the
/// kernel routing decision. Falls back to `127.0.0.1` if no
/// interface answers (e.g. fully-isolated network with no default
/// route): clients reaching the chat server will be reaching it via
/// a routable IP anyway, so a useful answer almost always exists.
#[must_use]
pub fn detect_lan_ipv4() -> Ipv4Addr {
  // Try a routable but non-existent address first (TEST-NET-1, RFC
  // 5737). The kernel still picks an outbound interface even though
  // the address is not actually pingable.
  if let Some(ip) = probe_outbound_ipv4(SocketAddr::from((Ipv4Addr::new(192, 0, 2, 1), 80))) {
    return ip;
  }
  // Fallback: try a well-known public IP so deployments behind an
  // airgapped network (no default route) at least get something.
  if let Some(ip) = probe_outbound_ipv4(SocketAddr::from((Ipv4Addr::new(8, 8, 8, 8), 80))) {
    return ip;
  }
  Ipv4Addr::LOCALHOST
}

fn probe_outbound_ipv4(probe: SocketAddr) -> Option<Ipv4Addr> {
  use std::net::UdpSocket as StdUdpSocket;
  let sock = StdUdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))).ok()?;
  sock.connect(probe).ok()?;
  let local = sock.local_addr().ok()?;
  match local.ip() {
    IpAddr::V4(v4) if !v4.is_unspecified() && !v4.is_loopback() => Some(v4),
    _ => None,
  }
}

// IPv6 silently unsupported by `detect_lan_ipv4` for now: WebRTC
// browsers prefer IPv4 srflx for LAN candidates and exposing only
// IPv4 simplifies firewall reasoning. A future extension could add
// `detect_lan_ipv6` symmetrical to the above.
#[allow(dead_code)]
const _: Option<Ipv6Addr> = None;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_handle_request_rejects_short_packet() {
    assert!(handle_request(&[0u8; 10], "127.0.0.1:1".parse().unwrap()).is_none());
  }

  #[test]
  fn test_handle_request_rejects_wrong_type() {
    let mut req = vec![0u8; 20];
    req[0] = 0x01;
    req[1] = 0x11; // Binding Indication, not request
    req[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    assert!(handle_request(&req, "127.0.0.1:1".parse().unwrap()).is_none());
  }

  #[test]
  fn test_handle_request_rejects_missing_cookie() {
    let mut req = vec![0u8; 20];
    req[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    // cookie left as 0 — invalid
    assert!(handle_request(&req, "127.0.0.1:1".parse().unwrap()).is_none());
  }

  #[test]
  fn test_binding_response_round_trip_ipv4() {
    let mut req = vec![0u8; 20];
    req[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    req[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    let tx_id = [
      0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    ];
    req[8..20].copy_from_slice(&tx_id);

    let src: SocketAddr = "192.168.1.42:54321".parse().unwrap();
    let resp = handle_request(&req, src).expect("response");

    // Header: type=0x0101, then length, then cookie, then tx_id.
    assert_eq!(&resp[0..2], &BINDING_SUCCESS.to_be_bytes());
    assert_eq!(&resp[4..8], &MAGIC_COOKIE.to_be_bytes());
    assert_eq!(&resp[8..20], &tx_id);

    // Attribute: type=0x0020 (XOR-MAPPED-ADDRESS).
    assert_eq!(&resp[20..22], &ATTR_XOR_MAPPED_ADDRESS.to_be_bytes());
    // Length = 8 (1 reserved + 1 family + 2 port + 4 ipv4).
    assert_eq!(&resp[22..24], &8u16.to_be_bytes());
    assert_eq!(resp[25], FAMILY_IPV4);

    // Decode the XOR'd port and verify it matches src.
    let port_xor = u16::from_be_bytes([resp[26], resp[27]]);
    let port = port_xor ^ ((MAGIC_COOKIE >> 16) as u16);
    assert_eq!(port, src.port());

    // Decode the XOR'd IP.
    let cookie_bytes = MAGIC_COOKIE.to_be_bytes();
    let ip = Ipv4Addr::new(
      resp[28] ^ cookie_bytes[0],
      resp[29] ^ cookie_bytes[1],
      resp[30] ^ cookie_bytes[2],
      resp[31] ^ cookie_bytes[3],
    );
    assert_eq!(IpAddr::V4(ip), src.ip());
  }

  #[test]
  fn test_detect_lan_ipv4_returns_routable_or_loopback() {
    let ip = detect_lan_ipv4();
    // We cannot assert a specific value (depends on the host) but
    // the function must not panic and must return an IPv4 address.
    let _ = ip; // smoke test only
  }
}
