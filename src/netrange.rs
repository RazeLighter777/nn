//! Pure IP-range arithmetic — no external crates, std::net only.
//!
//! The central use-case is computing which existing network a newly-scanned
//! IP (or set of IPs) belongs to, so that the import UI can propose a
//! sensible default instead of always offering "create new network".
//!
//! The "best" network is the one whose covering prefix is the **most
//! specific** (longest prefix-length) that still contains all of the
//! incoming addresses.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// ── Low-level prefix helpers ──────────────────────────────────────────────────

/// Build a 32-bit mask for an IPv4 prefix length (0–32).
/// prefix_len 0  → 0x0000_0000 (matches everything)
/// prefix_len 32 → 0xFFFF_FFFF (matches exactly one address)
fn v4_mask(prefix_len: u8) -> u32 {
    if prefix_len == 0 {
        0u32
    } else {
        !0u32 << (32 - prefix_len)
    }
}

/// Build a 128-bit mask for an IPv6 prefix length (0–128).
fn v6_mask(prefix_len: u8) -> u128 {
    if prefix_len == 0 {
        0u128
    } else {
        !0u128 << (128 - prefix_len)
    }
}

// ── Covering-prefix computation ───────────────────────────────────────────────

/// Compute the smallest (most-specific) IPv4 CIDR that covers every address
/// in `ips`.  Returns `None` if the slice is empty.
///
/// Algorithm: the prefix length equals the number of leading bits that are
/// identical between the smallest and largest address; the network address is
/// the minimum address masked to that prefix.
pub fn covering_prefix_v4(ips: &[Ipv4Addr]) -> Option<(Ipv4Addr, u8)> {
    let mut values = ips.iter().copied().map(u32::from);
    let first = values.next()?;
    let (lo, hi) = values.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let diff = lo ^ hi;
    let prefix_len: u8 = if diff == 0 { 32 } else { diff.leading_zeros() as u8 };
    let mask = v4_mask(prefix_len);
    Some((Ipv4Addr::from(lo & mask), prefix_len))
}

/// Compute the smallest IPv6 CIDR that covers every address in `ips`.
/// Returns `None` if the slice is empty.
pub fn covering_prefix_v6(ips: &[Ipv6Addr]) -> Option<(Ipv6Addr, u8)> {
    let mut values = ips.iter().copied().map(u128::from);
    let first = values.next()?;
    let (lo, hi) = values.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let diff = lo ^ hi;
    let prefix_len: u8 = if diff == 0 { 128 } else { diff.leading_zeros() as u8 };
    let mask = v6_mask(prefix_len);
    Some((Ipv6Addr::from(lo & mask), prefix_len))
}

/// Compute the covering CIDR for a homogeneous slice of `IpAddr` values.
///
/// Returns `None` if `ips` is empty or contains a mix of IPv4 and IPv6.
pub fn covering_cidr(ips: &[IpAddr]) -> Option<(IpAddr, u8)> {
    let v4s: Vec<Ipv4Addr> = ips
        .iter()
        .filter_map(|a| if let IpAddr::V4(v) = a { Some(*v) } else { None })
        .collect();
    let v6s: Vec<Ipv6Addr> = ips
        .iter()
        .filter_map(|a| if let IpAddr::V6(v) = a { Some(*v) } else { None })
        .collect();

    match (v4s.is_empty(), v6s.is_empty()) {
        (false, true) => covering_prefix_v4(&v4s).map(|(a, p)| (IpAddr::V4(a), p)),
        (true, false) => covering_prefix_v6(&v6s).map(|(a, p)| (IpAddr::V6(a), p)),
        _ => None, // empty or mixed
    }
}

// ── Containment test ──────────────────────────────────────────────────────────

/// Returns `true` when `ip` falls within the prefix `(net_addr, prefix_len)`.
///
/// IPv4 never matches an IPv6 prefix and vice versa.
pub fn ip_in_prefix(ip: IpAddr, net_addr: IpAddr, prefix_len: u8) -> bool {
    match (ip, net_addr) {
        (IpAddr::V4(ip4), IpAddr::V4(net4)) => {
            let mask = v4_mask(prefix_len.min(32));
            (u32::from(ip4) & mask) == (u32::from(net4) & mask)
        }
        (IpAddr::V6(ip6), IpAddr::V6(net6)) => {
            let mask = v6_mask(prefix_len.min(128));
            (u128::from(ip6) & mask) == (u128::from(net6) & mask)
        }
        _ => false,
    }
}

// ── CIDR formatting / parsing ─────────────────────────────────────────────────

/// Format a `(network_address, prefix_len)` pair as a CIDR string.
pub fn format_cidr(net_addr: IpAddr, prefix_len: u8) -> String {
    format!("{}/{}", net_addr, prefix_len)
}

// ── Subnet-aware covering prefix ─────────────────────────────────────────────

/// Like [`covering_cidr`] but each address carries its stored subnet prefix
/// length (the `netmask` column).
///
/// The *network address* for each member is computed as `ip & mask(prefix_len)`,
/// e.g. `192.168.88.1` with prefix_len 24 → `192.168.88.0`.  The result's
/// prefix_len is then capped at the smallest stored prefix_len so that a single
/// host recorded as `192.168.88.1/24` produces `192.168.88.0/24` rather than
/// the tight `192.168.88.1/32`.
///
/// Returns `None` for an empty or mixed-family slice.
pub fn covering_cidr_with_masks(inputs: &[(IpAddr, u8)]) -> Option<(IpAddr, u8)> {
    let v4s: Vec<(Ipv4Addr, u8)> = inputs
        .iter()
        .filter_map(|(a, p)| if let IpAddr::V4(v) = a { Some((*v, *p)) } else { None })
        .collect();
    let v6s: Vec<(Ipv6Addr, u8)> = inputs
        .iter()
        .filter_map(|(a, p)| if let IpAddr::V6(v) = a { Some((*v, *p)) } else { None })
        .collect();

    match (v4s.is_empty(), v6s.is_empty()) {
        (false, true) => {
            // Project each address onto its subnet network address.
            let net_addrs: Vec<Ipv4Addr> = v4s
                .iter()
                .map(|(ip, plen)| Ipv4Addr::from(u32::from(*ip) & v4_mask(*plen)))
                .collect();
            // Tightest spanning prefix of all those network addresses.
            let (span_net, span_prefix) = covering_prefix_v4(&net_addrs)?;
            // Don't go more specific than the narrowest stored prefix.
            let min_stored = v4s.iter().map(|(_, p)| *p).min().unwrap();
            let prefix_len = span_prefix.min(min_stored);
            let mask = v4_mask(prefix_len);
            Some((IpAddr::V4(Ipv4Addr::from(u32::from(span_net) & mask)), prefix_len))
        }
        (true, false) => {
            let net_addrs: Vec<Ipv6Addr> = v6s
                .iter()
                .map(|(ip, plen)| Ipv6Addr::from(u128::from(*ip) & v6_mask(*plen)))
                .collect();
            let (span_net, span_prefix) = covering_prefix_v6(&net_addrs)?;
            let min_stored = v6s.iter().map(|(_, p)| *p).min().unwrap();
            let prefix_len = span_prefix.min(min_stored);
            let mask = v6_mask(prefix_len);
            Some((IpAddr::V6(Ipv6Addr::from(u128::from(span_net) & mask)), prefix_len))
        }
        _ => None, // empty or mixed
    }
}

// ── Best-network selection ────────────────────────────────────────────────────

/// A network candidate carrying its computed covering prefix.
pub struct NetworkCoverage<'a> {
    /// Index into the original `NetworkOption` slice.
    pub index: usize,
    /// The computed `(net_addr, prefix_len)` derived from member addresses.
    pub prefix: (IpAddr, u8),
    /// Unused remainder (kept for callers that need the name etc.).
    pub _marker: std::marker::PhantomData<&'a ()>,
}

/// Given a list of `(network_addr, prefix_len)` candidates (one per existing
/// network) and a set of incoming IPs, return the index of the best-matching
/// network.
///
/// "Best" = the candidate with the **largest** prefix_len (most specific)
/// whose prefix contains **all** of the incoming parsed IPs.
///
/// Returns `None` if no candidate covers all incoming IPs.
pub fn best_matching_network(
    candidates: &[(IpAddr, u8)],  // (net_addr, prefix_len) per network, None absent
    incoming: &[IpAddr],
) -> Option<usize> {
    if incoming.is_empty() {
        return None;
    }
    candidates
        .iter()
        .enumerate()
        .filter(|(_, (net_addr, prefix_len))| {
            incoming
                .iter()
                .all(|ip| ip_in_prefix(*ip, *net_addr, *prefix_len))
        })
        .max_by_key(|(_, (_, prefix_len))| *prefix_len)
        .map(|(i, _)| i)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn v4(s: &str) -> Ipv4Addr {
        s.parse().unwrap()
    }
    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    // ── covering_prefix_v4 ────────────────────────────────────────────────────

    #[test]
    fn single_address_is_slash32() {
        let (net, len) = covering_prefix_v4(&[v4("10.0.0.1")]).unwrap();
        assert_eq!(len, 32);
        assert_eq!(net, v4("10.0.0.1"));
    }

    #[test]
    fn two_addrs_same_slash24() {
        let ips = [v4("192.168.88.1"), v4("192.168.88.200")];
        let (net, len) = covering_prefix_v4(&ips).unwrap();
        assert_eq!(len, 24, "should be /24");
        assert_eq!(net, v4("192.168.88.0"));
    }

    #[test]
    fn two_addrs_same_slash16() {
        let ips = [v4("10.1.0.1"), v4("10.1.255.254")];
        let (net, len) = covering_prefix_v4(&ips).unwrap();
        assert_eq!(len, 16, "should be /16");
        assert_eq!(net, v4("10.1.0.0"));
    }

    #[test]
    fn two_addrs_cross_slash23() {
        // 192.168.88.x and 192.168.89.x share a /23
        let ips = [v4("192.168.88.1"), v4("192.168.89.1")];
        let (net, len) = covering_prefix_v4(&ips).unwrap();
        assert_eq!(len, 23, "should be /23");
        assert_eq!(net, v4("192.168.88.0"));
    }

    #[test]
    fn slash0_covers_all() {
        let ips = [v4("0.0.0.0"), v4("255.255.255.255")];
        let (net, len) = covering_prefix_v4(&ips).unwrap();
        assert_eq!(len, 0);
        assert_eq!(net, v4("0.0.0.0"));
    }

    // ── ip_in_prefix ─────────────────────────────────────────────────────────

    #[test]
    fn ip_in_slash24() {
        let net = ip("192.168.88.0");
        assert!(ip_in_prefix(ip("192.168.88.1"), net, 24));
        assert!(ip_in_prefix(ip("192.168.88.254"), net, 24));
        assert!(!ip_in_prefix(ip("192.168.89.1"), net, 24));
    }

    #[test]
    fn ip_in_slash32_exact() {
        let net = ip("10.0.0.1");
        assert!(ip_in_prefix(ip("10.0.0.1"), net, 32));
        assert!(!ip_in_prefix(ip("10.0.0.2"), net, 32));
    }

    #[test]
    fn ip_in_slash0_always_true() {
        assert!(ip_in_prefix(ip("1.2.3.4"), ip("0.0.0.0"), 0));
        assert!(ip_in_prefix(ip("255.255.255.255"), ip("0.0.0.0"), 0));
    }

    #[test]
    fn v4_never_matches_v6_prefix() {
        assert!(!ip_in_prefix(ip("192.168.1.1"), ip("::1"), 128));
    }

    // ── best_matching_network ─────────────────────────────────────────────────

    #[test]
    fn picks_most_specific_network() {
        // Two candidates: /16 and /24 — both cover 192.168.88.5; prefer /24.
        let candidates = vec![
            (ip("192.168.0.0"), 16u8),
            (ip("192.168.88.0"), 24u8),
        ];
        let incoming = vec![ip("192.168.88.5")];
        assert_eq!(best_matching_network(&candidates, &incoming), Some(1));
    }

    #[test]
    fn no_match_returns_none() {
        let candidates = vec![(ip("10.0.0.0"), 8u8)];
        let incoming = vec![ip("192.168.1.1")];
        assert_eq!(best_matching_network(&candidates, &incoming), None);
    }

    #[test]
    fn all_ips_must_fit() {
        // One IP fits in /24, the other does not → no match.
        let candidates = vec![(ip("192.168.88.0"), 24u8)];
        let incoming = vec![ip("192.168.88.5"), ip("10.0.0.1")];
        assert_eq!(best_matching_network(&candidates, &incoming), None);
    }

    // ── covering_cidr_with_masks ──────────────────────────────────────────────

    #[test]
    fn single_v4_with_slash24_mask_expands_to_network() {
        // A host stored as 192.168.88.1/24 should produce 192.168.88.0/24,
        // not the tight 192.168.88.1/32.
        let inputs = vec![(IpAddr::V4(v4("192.168.88.1")), 24u8)];
        let (net, len) = covering_cidr_with_masks(&inputs).unwrap();
        assert_eq!(len, 24);
        assert_eq!(net, ip("192.168.88.0"));
    }

    #[test]
    fn two_v4_same_slash24_yields_slash24() {
        let inputs = vec![
            (IpAddr::V4(v4("192.168.88.1")), 24u8),
            (IpAddr::V4(v4("192.168.88.5")), 24u8),
        ];
        let (net, len) = covering_cidr_with_masks(&inputs).unwrap();
        assert_eq!(len, 24);
        assert_eq!(net, ip("192.168.88.0"));
    }

    #[test]
    fn narrowest_stored_prefix_caps_result() {
        // Both in same /24 but one stored as /16; result should be /16.
        let inputs = vec![
            (IpAddr::V4(v4("192.168.88.1")), 24u8),
            (IpAddr::V4(v4("192.168.88.2")), 16u8),
        ];
        let (net, len) = covering_cidr_with_masks(&inputs).unwrap();
        assert_eq!(len, 16);
        assert_eq!(net, ip("192.168.0.0"));
    }

    #[test]
    fn mixed_family_returns_none() {
        let inputs: Vec<(IpAddr, u8)> = vec![
            (ip("192.168.1.1"), 24u8),
            (ip("::1"), 128u8),
        ];
        assert!(covering_cidr_with_masks(&inputs).is_none());
    }
}
