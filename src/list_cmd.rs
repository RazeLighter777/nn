use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use regex::Regex;

use crate::{
    AnyConnection, Args, Commands, ListingTypes, NNError, ResourceTypesFilters,
    establish_connection, models, schema,
};

// ── Output format ─────────────────────────────────────────────────────────────

pub enum OutputFormat {
    /// Print just IP addresses, one per line (default).
    Addresses,
    /// Print nmap-ready argument strings: `-p <ports> <targets>`.
    NmapArgs,
    /// Print human-readable details for each matched resource.
    HumanReadable,
    /// Print ID numbers only, one per line (for scripting)
    Ids,
}

impl OutputFormat {
    fn from_listing_types(lt: &ListingTypes) -> Self {
        if lt.nmap_args {
            Self::NmapArgs
        } else if lt.readable {
            Self::HumanReadable
        }  else if lt.ids {
            Self::Ids
        }
        else {
            Self::Addresses
        }
    }
}

// ── Pattern filter ────────────────────────────────────────────────────────────

/// Compiled AND-filter: every pattern must match at least one candidate string.
/// Empty filter matches everything.
pub struct PatternFilter {
    patterns: Vec<Regex>,
}

impl PatternFilter {
    pub fn compile(raw: &[String]) -> Result<Self, NNError> {
        let patterns = raw
            .iter()
            .map(|s| Regex::new(s).map_err(|e| NNError::InvalidRegex(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { patterns })
    }

    /// Returns `true` when every pattern matches at least one of `candidates`
    /// (AND across patterns, OR across candidates).
    pub fn matches(&self, candidates: &[&str]) -> bool {
        self.patterns
            .iter()
            .all(|re| candidates.iter().any(|c| re.is_match(c)))
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

// ── Port helpers ──────────────────────────────────────────────────────────────

fn parse_ports(raw: &[String]) -> Vec<i32> {
    raw.iter()
        .flat_map(|s| s.split(','))
        .filter_map(|p| p.trim().parse::<i32>().ok())
        .collect()
}

// ── Site resolution ───────────────────────────────────────────────────────────

fn resolve_site_id(conn: &mut AnyConnection, site: Option<&str>) -> Result<Option<i32>, NNError> {
    let Some(name) = site else {
        return Ok(None);
    };
    use schema::site::dsl as s;
    let row = s::site
        .filter(s::name.eq(name))
        .select(s::id)
        .first::<i32>(conn)
        .optional()?;
    Ok(row)
}

// ── Tag helpers ───────────────────────────────────────────────────────────────

/// Resolve tag names → IDs. Returns `None` if any requested tag does not exist
/// (meaning the result set would be empty regardless).
fn resolve_tag_ids(
    conn: &mut AnyConnection,
    names: &[String],
) -> Result<Option<Vec<i32>>, NNError> {
    if names.is_empty() {
        return Ok(Some(vec![]));
    }
    use schema::tag::dsl as t;
    let mut ids = Vec::new();
    for name in names {
        let row = t::tag
            .filter(t::name.eq(name))
            .select(t::id)
            .first::<i32>(conn)
            .optional()?;
        match row {
            Some(id) => ids.push(id),
            None => return Ok(None),
        }
    }
    Ok(Some(ids))
}

/// Return host IDs that carry ALL of the given tag IDs.
fn tagged_host_ids(conn: &mut AnyConnection, tag_ids: &[i32]) -> Result<Vec<i32>, NNError> {
    use schema::tag_assignment::dsl as ta;
    let mut result: Option<Vec<i32>> = None;
    for &tid in tag_ids {
        let ids: Vec<i32> = ta::tag_assignment
            .filter(ta::tag_id.eq(tid))
            .filter(ta::host_id.is_not_null())
            .select(ta::host_id.assume_not_null())
            .load(conn)?;
        result = Some(match result {
            None => ids,
            Some(prev) => prev.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    Ok(result.unwrap_or_default())
}

/// Return address IDs that carry ALL of the given tag IDs.
fn tagged_address_ids(conn: &mut AnyConnection, tag_ids: &[i32]) -> Result<Vec<i32>, NNError> {
    use schema::tag_assignment::dsl as ta;
    let mut result: Option<Vec<i32>> = None;
    for &tid in tag_ids {
        let ids: Vec<i32> = ta::tag_assignment
            .filter(ta::tag_id.eq(tid))
            .filter(ta::address_id.is_not_null())
            .select(ta::address_id.assume_not_null())
            .load(conn)?;
        result = Some(match result {
            None => ids,
            Some(prev) => prev.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    Ok(result.unwrap_or_default())
}

/// Return network IDs that carry ALL of the given tag IDs.
fn tagged_network_ids(conn: &mut AnyConnection, tag_ids: &[i32]) -> Result<Vec<i32>, NNError> {
    use schema::tag_assignment::dsl as ta;
    let mut result: Option<Vec<i32>> = None;
    for &tid in tag_ids {
        let ids: Vec<i32> = ta::tag_assignment
            .filter(ta::tag_id.eq(tid))
            .filter(ta::network_id.is_not_null())
            .select(ta::network_id.assume_not_null())
            .load(conn)?;
        result = Some(match result {
            None => ids,
            Some(prev) => prev.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    Ok(result.unwrap_or_default())
}

/// Return service IDs that carry ALL of the given tag IDs.
fn tagged_service_ids(conn: &mut AnyConnection, tag_ids: &[i32]) -> Result<Vec<i32>, NNError> {
    use schema::tag_assignment::dsl as ta;
    let mut result: Option<Vec<i32>> = None;
    for &tid in tag_ids {
        let ids: Vec<i32> = ta::tag_assignment
            .filter(ta::tag_id.eq(tid))
            .filter(ta::service_id.is_not_null())
            .select(ta::service_id.assume_not_null())
            .load(conn)?;
        result = Some(match result {
            None => ids,
            Some(prev) => prev.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    Ok(result.unwrap_or_default())
}

/// Return credential IDs that carry ALL of the given tag IDs.
fn tagged_credential_ids(conn: &mut AnyConnection, tag_ids: &[i32]) -> Result<Vec<i32>, NNError> {
    use schema::tag_assignment::dsl as ta;
    let mut result: Option<Vec<i32>> = None;
    for &tid in tag_ids {
        let ids: Vec<i32> = ta::tag_assignment
            .filter(ta::tag_id.eq(tid))
            .filter(ta::credential_id.is_not_null())
            .select(ta::credential_id.assume_not_null())
            .load(conn)?;
        result = Some(match result {
            None => ids,
            Some(prev) => prev.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    Ok(result.unwrap_or_default())
}

// ── Notes / tags helpers for display ─────────────────────────────────────────

/// Returns the set of host IDs (from `host_ids`) that have at least one note.
fn noted_host_ids(conn: &mut AnyConnection, host_ids: &[i32]) -> Result<HashSet<i32>, NNError> {
    if host_ids.is_empty() { return Ok(Default::default()); }
    let set: HashSet<i32> = host_ids.iter().copied().collect();
    let all: Vec<i32> = schema::note::table
        .filter(schema::note::host_id.is_not_null())
        .select(schema::note::host_id.assume_not_null())
        .load(conn)?;
    Ok(all.into_iter().filter(|id| set.contains(id)).collect())
}

fn noted_address_ids(conn: &mut AnyConnection, addr_ids: &[i32]) -> Result<HashSet<i32>, NNError> {
    if addr_ids.is_empty() { return Ok(Default::default()); }
    let set: HashSet<i32> = addr_ids.iter().copied().collect();
    let all: Vec<i32> = schema::note::table
        .filter(schema::note::address_id.is_not_null())
        .select(schema::note::address_id.assume_not_null())
        .load(conn)?;
    Ok(all.into_iter().filter(|id| set.contains(id)).collect())
}

fn noted_network_ids(conn: &mut AnyConnection, net_ids: &[i32]) -> Result<HashSet<i32>, NNError> {
    if net_ids.is_empty() { return Ok(Default::default()); }
    let set: HashSet<i32> = net_ids.iter().copied().collect();
    let all: Vec<i32> = schema::note::table
        .filter(schema::note::network_id.is_not_null())
        .select(schema::note::network_id.assume_not_null())
        .load(conn)?;
    Ok(all.into_iter().filter(|id| set.contains(id)).collect())
}

fn noted_service_ids(conn: &mut AnyConnection, svc_ids: &[i32]) -> Result<HashSet<i32>, NNError> {
    if svc_ids.is_empty() { return Ok(Default::default()); }
    let set: HashSet<i32> = svc_ids.iter().copied().collect();
    let all: Vec<i32> = schema::note::table
        .filter(schema::note::service_id.is_not_null())
        .select(schema::note::service_id.assume_not_null())
        .load(conn)?;
    Ok(all.into_iter().filter(|id| set.contains(id)).collect())
}

fn noted_credential_ids(conn: &mut AnyConnection, cred_ids: &[i32]) -> Result<HashSet<i32>, NNError> {
    if cred_ids.is_empty() { return Ok(Default::default()); }
    let set: HashSet<i32> = cred_ids.iter().copied().collect();
    let all: Vec<i32> = schema::note::table
        .filter(schema::note::credential_id.is_not_null())
        .select(schema::note::credential_id.assume_not_null())
        .load(conn)?;
    Ok(all.into_iter().filter(|id| set.contains(id)).collect())
}

/// Returns a map resource_id -> tag names for the given (resource_id, tag_id) pairs.
fn build_tag_map(pairs: Vec<(i32, i32)>, all_tags: &[models::Tag]) -> HashMap<i32, Vec<String>> {
    let tag_by_id: HashMap<i32, &str> =
        all_tags.iter().map(|t| (t.id, t.name.as_str())).collect();
    let mut result: HashMap<i32, Vec<String>> = HashMap::new();
    for (rid, tid) in pairs {
        if let Some(&name) = tag_by_id.get(&tid) {
            result.entry(rid).or_default().push(name.to_string());
        }
    }
    result
}

fn load_host_tags(
    conn: &mut AnyConnection,
    host_ids: &[i32],
) -> Result<HashMap<i32, Vec<String>>, NNError> {
    if host_ids.is_empty() { return Ok(Default::default()); }
    let id_set: HashSet<i32> = host_ids.iter().copied().collect();
    let pairs: Vec<(i32, i32)> = schema::tag_assignment::table
        .filter(schema::tag_assignment::host_id.is_not_null())
        .select((
            schema::tag_assignment::host_id.assume_not_null(),
            schema::tag_assignment::tag_id,
        ))
        .load(conn)?;
    let pairs: Vec<(i32, i32)> = pairs.into_iter().filter(|(hid, _)| id_set.contains(hid)).collect();
    let tag_ids: Vec<i32> = pairs.iter().map(|(_, tid)| *tid).collect();
    let all_tags: Vec<models::Tag> = schema::tag::table
        .filter(schema::tag::id.eq_any(&tag_ids))
        .load(conn)?;
    Ok(build_tag_map(pairs, &all_tags))
}

fn load_address_tags(
    conn: &mut AnyConnection,
    addr_ids: &[i32],
) -> Result<HashMap<i32, Vec<String>>, NNError> {
    if addr_ids.is_empty() { return Ok(Default::default()); }
    let id_set: HashSet<i32> = addr_ids.iter().copied().collect();
    let pairs: Vec<(i32, i32)> = schema::tag_assignment::table
        .filter(schema::tag_assignment::address_id.is_not_null())
        .select((
            schema::tag_assignment::address_id.assume_not_null(),
            schema::tag_assignment::tag_id,
        ))
        .load(conn)?;
    let pairs: Vec<(i32, i32)> = pairs.into_iter().filter(|(aid, _)| id_set.contains(aid)).collect();
    let tag_ids: Vec<i32> = pairs.iter().map(|(_, tid)| *tid).collect();
    let all_tags: Vec<models::Tag> = schema::tag::table
        .filter(schema::tag::id.eq_any(&tag_ids))
        .load(conn)?;
    Ok(build_tag_map(pairs, &all_tags))
}

fn load_network_tags(
    conn: &mut AnyConnection,
    net_ids: &[i32],
) -> Result<HashMap<i32, Vec<String>>, NNError> {
    if net_ids.is_empty() { return Ok(Default::default()); }
    let id_set: HashSet<i32> = net_ids.iter().copied().collect();
    let pairs: Vec<(i32, i32)> = schema::tag_assignment::table
        .filter(schema::tag_assignment::network_id.is_not_null())
        .select((
            schema::tag_assignment::network_id.assume_not_null(),
            schema::tag_assignment::tag_id,
        ))
        .load(conn)?;
    let pairs: Vec<(i32, i32)> = pairs.into_iter().filter(|(nid, _)| id_set.contains(nid)).collect();
    let tag_ids: Vec<i32> = pairs.iter().map(|(_, tid)| *tid).collect();
    let all_tags: Vec<models::Tag> = schema::tag::table
        .filter(schema::tag::id.eq_any(&tag_ids))
        .load(conn)?;
    Ok(build_tag_map(pairs, &all_tags))
}

fn load_service_tags(
    conn: &mut AnyConnection,
    svc_ids: &[i32],
) -> Result<HashMap<i32, Vec<String>>, NNError> {
    if svc_ids.is_empty() { return Ok(Default::default()); }
    let id_set: HashSet<i32> = svc_ids.iter().copied().collect();
    let pairs: Vec<(i32, i32)> = schema::tag_assignment::table
        .filter(schema::tag_assignment::service_id.is_not_null())
        .select((
            schema::tag_assignment::service_id.assume_not_null(),
            schema::tag_assignment::tag_id,
        ))
        .load(conn)?;
    let pairs: Vec<(i32, i32)> = pairs.into_iter().filter(|(sid, _)| id_set.contains(sid)).collect();
    let tag_ids: Vec<i32> = pairs.iter().map(|(_, tid)| *tid).collect();
    let all_tags: Vec<models::Tag> = schema::tag::table
        .filter(schema::tag::id.eq_any(&tag_ids))
        .load(conn)?;
    Ok(build_tag_map(pairs, &all_tags))
}

fn load_credential_tags(
    conn: &mut AnyConnection,
    cred_ids: &[i32],
) -> Result<HashMap<i32, Vec<String>>, NNError> {
    if cred_ids.is_empty() { return Ok(Default::default()); }
    let id_set: HashSet<i32> = cred_ids.iter().copied().collect();
    let pairs: Vec<(i32, i32)> = schema::tag_assignment::table
        .filter(schema::tag_assignment::credential_id.is_not_null())
        .select((
            schema::tag_assignment::credential_id.assume_not_null(),
            schema::tag_assignment::tag_id,
        ))
        .load(conn)?;
    let pairs: Vec<(i32, i32)> = pairs.into_iter().filter(|(cid, _)| id_set.contains(cid)).collect();
    let tag_ids: Vec<i32> = pairs.iter().map(|(_, tid)| *tid).collect();
    let all_tags: Vec<models::Tag> = schema::tag::table
        .filter(schema::tag::id.eq_any(&tag_ids))
        .load(conn)?;
    Ok(build_tag_map(pairs, &all_tags))
}

/// Format a tags list like `[web, prod]` or empty string.
fn fmt_tags(tags: Option<&Vec<String>>) -> String {
    match tags {
        Some(t) if !t.is_empty() => format!(" [{}]", t.join(", ")),
        _ => String::new(),
    }
}


// ── nmap output helper ────────────────────────────────────────────────────────

/// Group `(port, ip)` pairs and print one `-p <port> <ip …>` line per unique port.
fn print_nmap_args(port_ip_pairs: &[(i32, String)]) {
    use std::collections::BTreeMap;
    let mut by_port: BTreeMap<i32, Vec<&str>> = BTreeMap::new();
    for (port, ip) in port_ip_pairs {
        by_port.entry(*port).or_default().push(ip.as_str());
    }
    for (port, ips) in &by_port {
        // Deduplicate IPs for each port
        let mut unique: Vec<&str> = ips.clone();
        unique.dedup();
        println!("-p {} {}", port, unique.join(" "));
    }
}

/// Print a service row with the given line prefix.
fn print_service_human(svc: &models::Service, indent: &str) {
    println!(
        "{}service: {} port={} proto={} state={}",
        indent, svc.name, svc.port, svc.ip_proto_number, svc.state
    );
    if let Some(ref p) = svc.product {
        println!("{}  product: {}", indent, p);
    }
    if let Some(ref v) = svc.version {
        println!("{}  version: {}", indent, v);
    }
    if let Some(ref e) = svc.extra_info {
        println!("{}  extra:   {}", indent, e);
    }
}

// ── list hosts ────────────────────────────────────────────────────────────────

fn list_hosts(
    conn: &mut AnyConnection,
    site_id: Option<i32>,
    tags: &[String],
    patterns: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    use schema::{address, host, service};

    let pf = PatternFilter::compile(patterns)?;
    let tag_ids = resolve_tag_ids(conn, tags)?;
    let Some(tag_ids) = tag_ids else {
        return Ok(());
    };

    let hosts: Vec<models::Host> = {
        let mut q = host::table.into_boxed();
        if let Some(sid) = site_id {
            q = q.filter(host::site_id.eq(sid));
        }
        if !tag_ids.is_empty() {
            let tagged = tagged_host_ids(conn, &tag_ids)?;
            q = q.filter(host::id.eq_any(tagged));
        }
        q.load(conn)?
    };

    if hosts.is_empty() {
        return Ok(());
    }

    let host_ids: Vec<i32> = hosts.iter().map(|h| h.id).collect();
    let all_addresses: Vec<models::Addres> =
        address::table.filter(address::host_id.eq_any(&host_ids)).load(conn)?;
    let addr_ids: Vec<i32> = all_addresses.iter().map(|a| a.id).collect();
    let all_services: Vec<models::Service> = service::table
        .select(models::Service::as_select())
        .filter(service::address_id.eq_any(&addr_ids))
        .load(conn)?;

    // For human-readable: load notes existence and tags once for all hosts
    let (host_noted, host_tag_map) = if let OutputFormat::HumanReadable = fmt {
        (
            noted_host_ids(conn, &host_ids)?,
            load_host_tags(conn, &host_ids)?,
        )
    } else {
        (HashSet::new(), HashMap::new())
    };

    for host in &hosts {
        let host_addrs: Vec<&models::Addres> =
            all_addresses.iter().filter(|a| a.host_id == host.id).collect();
        let host_ips: Vec<&str> = host_addrs.iter().map(|a| a.ip.as_str()).collect();

        if !pf.is_empty() {
            let mut candidates: Vec<&str> = vec![host.name.as_str()];
            if let Some(ref hn) = host.hostname {
                candidates.push(hn.as_str());
            }
            candidates.extend_from_slice(&host_ips);
            if !pf.matches(&candidates) {
                continue;
            }
        }

        match fmt {
            OutputFormat::Addresses => {
                for ip in &host_ips {
                    println!("{}", ip);
                }
            }
            OutputFormat::NmapArgs => {
                let pairs: Vec<(i32, String)> = all_services
                    .iter()
                    .filter(|s| host_addrs.iter().any(|a| a.id == s.address_id))
                    .flat_map(|s| {
                        let ip = all_addresses
                            .iter()
                            .find(|a| a.id == s.address_id)
                            .map(|a| a.ip.clone())
                            .unwrap_or_default();
                        std::iter::once((s.port, ip))
                    })
                    .collect();
                print_nmap_args(&pairs);
            }
            OutputFormat::HumanReadable => {
                let note_marker = if host_noted.contains(&host.id) { " *" } else { "" };
                let tags = fmt_tags(host_tag_map.get(&host.id));
                println!("host: {}{} (id={}){}", host.name, note_marker, host.id, tags);
                if let Some(ref hn) = host.hostname {
                    println!("  hostname: {}", hn);
                }
                if let Some(ref os) = host.os_type {
                    println!("  os:       {}", os);
                }
                for addr in &host_addrs {
                    println!("  address:  {}/{}", addr.ip, addr.netmask);
                    if let Some(ref mac) = addr.mac {
                        println!("    mac:    {}", mac);
                    }
                    let addr_services: Vec<&models::Service> =
                        all_services.iter().filter(|s| s.address_id == addr.id).collect();
                    for svc in addr_services {
                        print_service_human(svc, "    ");
                    }
                }
            }
            OutputFormat::Ids => {
                eprintln!("host: (hostname: {}) {}", host.hostname.as_deref().unwrap_or(""), host.name);
                println!("{}", host.id);
            }

        }
    }
    Ok(())
}

// ── list addresses ────────────────────────────────────────────────────────────

fn list_addresses(
    conn: &mut AnyConnection,
    site_id: Option<i32>,
    tags: &[String],
    patterns: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    use schema::{address, host, service};

    let pf = PatternFilter::compile(patterns)?;
    let tag_ids = resolve_tag_ids(conn, tags)?;
    let Some(tag_ids) = tag_ids else {
        return Ok(());
    };

    let addresses: Vec<models::Addres> = {
        let mut q = address::table.into_boxed();
        if let Some(sid) = site_id {
            let host_ids: Vec<i32> = host::table
                .filter(host::site_id.eq(sid))
                .select(host::id)
                .load(conn)?;
            q = q.filter(address::host_id.eq_any(host_ids));
        }
        if !tag_ids.is_empty() {
            let tagged = tagged_address_ids(conn, &tag_ids)?;
            q = q.filter(address::id.eq_any(tagged));
        }
        q.load(conn)?
    };

    if addresses.is_empty() {
        return Ok(());
    }

    let addr_ids: Vec<i32> = addresses.iter().map(|a| a.id).collect();
    let all_services: Vec<models::Service> = service::table
        .select(models::Service::as_select())
        .filter(service::address_id.eq_any(&addr_ids))
        .load(conn)?;

    // For human-readable: load notes existence and tags once for all addresses
    let (addr_noted, addr_tag_map) = if let OutputFormat::HumanReadable = fmt {
        (
            noted_address_ids(conn, &addr_ids)?,
            load_address_tags(conn, &addr_ids)?,
        )
    } else {
        (HashSet::new(), HashMap::new())
    };

    for addr in &addresses {
        let mut candidates: Vec<&str> = vec![addr.ip.as_str()];
        if let Some(ref mac) = addr.mac {
            candidates.push(mac.as_str());
        }
        if !pf.is_empty() && !pf.matches(&candidates) {
            continue;
        }

        match fmt {
            OutputFormat::Addresses => println!("{}", addr.ip),
            OutputFormat::NmapArgs => {
                let pairs: Vec<(i32, String)> = all_services
                    .iter()
                    .filter(|s| s.address_id == addr.id)
                    .map(|s| (s.port, addr.ip.clone()))
                    .collect();
                print_nmap_args(&pairs);
            }
            OutputFormat::HumanReadable => {
                let note_marker = if addr_noted.contains(&addr.id) { " *" } else { "" };
                let tags = fmt_tags(addr_tag_map.get(&addr.id));
                println!("address: {}/{}{} (id={}){}", addr.ip, addr.netmask, note_marker, addr.id, tags);
                if let Some(ref mac) = addr.mac {
                    println!("  mac: {}", mac);
                }
                let addr_svcs: Vec<&models::Service> =
                    all_services.iter().filter(|s| s.address_id == addr.id).collect();
                for svc in addr_svcs {
                    print_service_human(svc, "  ");
                }
            }
            OutputFormat::Ids => {
                eprintln!("address: {}/{}", addr.ip, addr.netmask);
                println!("{}", addr.id);
            }
        }
    }
    Ok(())
}

// ── list networks ─────────────────────────────────────────────────────────────

fn list_networks(
    conn: &mut AnyConnection,
    site_id: Option<i32>,
    tags: &[String],
    patterns: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    use schema::{address, network, service};

    let pf = PatternFilter::compile(patterns)?;
    let tag_ids = resolve_tag_ids(conn, tags)?;
    let Some(tag_ids) = tag_ids else {
        return Ok(());
    };

    let networks: Vec<models::Network> = {
        let mut q = network::table.into_boxed();
        if let Some(sid) = site_id {
            q = q.filter(network::site_id.eq(sid));
        }
        if !tag_ids.is_empty() {
            let tagged = tagged_network_ids(conn, &tag_ids)?;
            q = q.filter(network::id.eq_any(tagged));
        }
        q.load(conn)?
    };

    if networks.is_empty() {
        return Ok(());
    }

    let net_ids: Vec<i32> = networks.iter().map(|n| n.id).collect();
    // early case for if output is ids only - we can skip loading services and addresses
    if let OutputFormat::Ids = fmt {
        for net in &networks {
            eprintln!("network: {} (id={})", net.name, net.id);
            println!("{}", net.id);
        }
        return Ok(());
    }
    let all_addresses: Vec<models::Addres> =
        address::table.filter(address::network_id.eq_any(&net_ids)).load(conn)?;
    let addr_ids: Vec<i32> = all_addresses.iter().map(|a| a.id).collect();
    let all_services: Vec<models::Service> = service::table
        .select(models::Service::as_select())
        .filter(service::address_id.eq_any(&addr_ids))
        .load(conn)?;

    // For human-readable: load notes existence and tags once for all networks
    let (net_noted, net_tag_map) = if let OutputFormat::HumanReadable = fmt {
        (
            noted_network_ids(conn, &net_ids)?,
            load_network_tags(conn, &net_ids)?,
        )
    } else {
        (HashSet::new(), HashMap::new())
    };

    for net in &networks {
        let net_addrs: Vec<&models::Addres> =
            all_addresses.iter().filter(|a| a.network_id == net.id).collect();
        let net_ips: Vec<&str> = net_addrs.iter().map(|a| a.ip.as_str()).collect();

        if !pf.is_empty() {
            let mut candidates: Vec<&str> = vec![net.name.as_str()];
            candidates.extend_from_slice(&net_ips);
            if !pf.matches(&candidates) {
                continue;
            }
        }

        match fmt {
            OutputFormat::Addresses => {
                // Emit each address as CIDR notation
                for addr in &net_addrs {
                    println!("{}/{}", addr.ip, addr.netmask);
                }
            }
            OutputFormat::NmapArgs => {
                let pairs: Vec<(i32, String)> = all_services
                    .iter()
                    .filter(|s| net_addrs.iter().any(|a| a.id == s.address_id))
                    .map(|s| {
                        let ip = all_addresses
                            .iter()
                            .find(|a| a.id == s.address_id)
                            .map(|a| a.ip.clone())
                            .unwrap_or_default();
                        (s.port, ip)
                    })
                    .collect();
                print_nmap_args(&pairs);
            }
            OutputFormat::HumanReadable => {
                let note_marker = if net_noted.contains(&net.id) { " *" } else { "" };
                let tags = fmt_tags(net_tag_map.get(&net.id));
                println!("network: {}{} (id={}){}", net.name, note_marker, net.id, tags);
                for addr in &net_addrs {
                    println!("  address: {}/{}", addr.ip, addr.netmask);
                    if let Some(ref mac) = addr.mac {
                        println!("    mac: {}", mac);
                    }
                    let addr_svcs: Vec<&models::Service> =
                        all_services.iter().filter(|s| s.address_id == addr.id).collect();
                    for svc in addr_svcs {
                        print_service_human(svc, "    ");
                    }
                }
            }
            OutputFormat::Ids=> {
                // We already handled this case above with an early return, but we need to have it here to satisfy the match exhaustiveness check. It won't be reached due to the early return.
                unreachable!();
             }
        }
    }
    Ok(())
}

// ── list services ─────────────────────────────────────────────────────────────

fn list_services(
    conn: &mut AnyConnection,
    site_id: Option<i32>,
    tags: &[String],
    patterns: &[String],
    port_strs: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    use schema::{address, service};

    let pf = PatternFilter::compile(patterns)?;
    let port_filter = parse_ports(port_strs);
    let tag_ids = resolve_tag_ids(conn, tags)?;
    let Some(tag_ids) = tag_ids else {
        return Ok(());
    };

    let services: Vec<models::Service> = {
        let mut q = service::table
            .select(models::Service::as_select())
            .into_boxed();
        if let Some(sid) = site_id {
            q = q.filter(service::site_id.eq(sid));
        }
        if !port_filter.is_empty() {
            q = q.filter(service::port.eq_any(&port_filter));
        }
        if !tag_ids.is_empty() {
            let tagged = tagged_service_ids(conn, &tag_ids)?;
            q = q.filter(service::id.eq_any(tagged));
        }
        q.load(conn)?
    };

    if services.is_empty() {
        return Ok(());
    }

    let addr_ids: Vec<i32> = services.iter().map(|s| s.address_id).collect();
    let all_addresses: Vec<models::Addres> =
        address::table.filter(address::id.eq_any(&addr_ids)).load(conn)?;

    // For human-readable: load notes existence and tags once for all services
    let svc_ids: Vec<i32> = services.iter().map(|s| s.id).collect();
    let (svc_noted, svc_tag_map) = if let OutputFormat::HumanReadable = fmt {
        (
            noted_service_ids(conn, &svc_ids)?,
            load_service_tags(conn, &svc_ids)?,
        )
    } else {
        (HashSet::new(), HashMap::new())
    };

    for svc in &services {
        let service_ip = all_addresses
            .iter()
            .find(|a| a.id == svc.address_id)
            .map(|a| a.ip.as_str())
            .unwrap_or("unknown");

        if !pf.is_empty() {
            let mut candidates: Vec<&str> = vec![svc.name.as_str(), service_ip];
            if let Some(ref p) = svc.product {
                candidates.push(p.as_str());
            }
            if let Some(ref v) = svc.version {
                candidates.push(v.as_str());
            }
            if !pf.matches(&candidates) {
                continue;
            }
        }

        match fmt {
            OutputFormat::Addresses => println!("{}", service_ip),
            OutputFormat::NmapArgs => println!("-p {} {}", svc.port, service_ip),
            OutputFormat::HumanReadable => {
                let note_marker = if svc_noted.contains(&svc.id) { " *" } else { "" };
                let tags = fmt_tags(svc_tag_map.get(&svc.id));
                println!(
                    "service: {}:{} @ {}{} (id={}){}", 
                    svc.name, svc.port, service_ip, note_marker, svc.id, tags
                );
                print_service_human(svc, "  ");
            }
            OutputFormat::Ids => {
                eprintln!(
                    "service: {}:{} @ {}:",
                    svc.name, svc.port, service_ip,
                );
                println!("{}", svc.id);
            }
        }
    }
    Ok(())
}

// ── list notes ────────────────────────────────────────────────────────────────
fn list_notes(
    conn: &mut AnyConnection,
    _site_id: Option<i32>,
    _tags: &[String],
    patterns: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    use schema::{address, note};

    let pf = PatternFilter::compile(patterns)?;
    let notes: Vec<models::Note> = note::table.load(conn)?;

    let note_addr_ids: Vec<i32> = notes.iter().filter_map(|n| n.address_id).collect();
    let note_addrs: Vec<models::Addres> = if note_addr_ids.is_empty() {
        vec![]
    } else {
        address::table.filter(address::id.eq_any(&note_addr_ids)).load(conn)?
    };

    for note in &notes {
        if !pf.is_empty() && !pf.matches(&[note.text.as_str()]) {
            continue;
        }

        let assoc_addr = note
            .address_id
            .and_then(|aid| note_addrs.iter().find(|a| a.id == aid));

        match fmt {
            OutputFormat::Addresses => {
                if let Some(addr) = assoc_addr {
                    println!("{}", addr.ip);
                }
            }
            OutputFormat::NmapArgs => {
                // Notes have no port; emit the associated IP if available
                if let Some(addr) = assoc_addr {
                    println!("{}", addr.ip);
                }
            }
            OutputFormat::HumanReadable => {
                println!("note (id={}): {}", note.id, note.text);
                if let Some(addr) = assoc_addr {
                    println!("  address:    {}", addr.ip);
                }
                if let Some(hid) = note.host_id {
                    println!("  host_id:    {}", hid);
                }
                if let Some(sid) = note.service_id {
                    println!("  service_id: {}", sid);
                }
                if let Some(nid) = note.network_id {
                    println!("  network_id: {}", nid);
                }
            }
            OutputFormat::Ids => {
                println!("{}", note.id);
            }
        }
    }
    Ok(())
}

// ── list tags ─────────────────────────────────────────────────────────────────

fn list_tags(
    conn: &mut AnyConnection,
    patterns: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    let pf = PatternFilter::compile(patterns)?;
    let tags: Vec<models::Tag> = schema::tag::table.load(conn)?;

    for tag in &tags {
        if !pf.is_empty() && !pf.matches(&[tag.name.as_str()]) {
            continue;
        }
        match fmt {
            OutputFormat::HumanReadable | OutputFormat::Addresses | OutputFormat::NmapArgs => {
                println!("tag: {} (id={})", tag.name, tag.id);
            }
            OutputFormat::Ids => {
                eprintln!("tag: {}", tag.name);
                println!("{}", tag.id);
            }
        }
    }
    Ok(())
}

// ── list credentials ──────────────────────────────────────────────────────────

fn list_credentials(
    conn: &mut AnyConnection,
    tags: &[String],
    patterns: &[String],
    fmt: &OutputFormat,
) -> Result<(), NNError> {
    use schema::{credential, credential_service, service, address};

    let pf = PatternFilter::compile(patterns)?;
    let tag_ids = resolve_tag_ids(conn, tags)?;
    let Some(tag_ids) = tag_ids else {
        return Ok(());
    };

    let creds: Vec<models::Credential> = {
        let mut q = credential::table
            .select(models::Credential::as_select())
            .into_boxed();
        if !tag_ids.is_empty() {
            let tagged = tagged_credential_ids(conn, &tag_ids)?;
            q = q.filter(credential::id.eq_any(tagged));
        }
        q.load(conn)?
    };

    if creds.is_empty() {
        return Ok(());
    }

    let cred_ids: Vec<i32> = creds.iter().map(|c| c.id).collect();

    // Load all credential_service rows for these credentials
    let cs_rows: Vec<models::CredentialService> = credential_service::table
        .filter(credential_service::credential_id.eq_any(&cred_ids))
        .load(conn)?;

    let svc_ids: Vec<i32> = cs_rows.iter().map(|cs| cs.service_id).collect();
    let all_services: Vec<models::Service> = if svc_ids.is_empty() {
        vec![]
    } else {
        service::table
            .select(models::Service::as_select())
            .filter(service::id.eq_any(&svc_ids))
            .load(conn)?
    };

    let svc_addr_ids: Vec<i32> = all_services.iter().map(|s| s.address_id).collect();
    let all_addresses: Vec<models::Addres> = if svc_addr_ids.is_empty() {
        vec![]
    } else {
        address::table.filter(address::id.eq_any(&svc_addr_ids)).load(conn)?
    };

    let (cred_noted, cred_tag_map) = if let OutputFormat::HumanReadable = fmt {
        (
            noted_credential_ids(conn, &cred_ids)?,
            load_credential_tags(conn, &cred_ids)?,
        )
    } else {
        (HashSet::new(), HashMap::new())
    };

    for cred in &creds {
        let assoc_svcs: Vec<&models::Service> = cs_rows
            .iter()
            .filter(|cs| cs.credential_id == cred.id)
            .filter_map(|cs| all_services.iter().find(|s| s.id == cs.service_id))
            .collect();

        // Pattern filter: match username or associated service name/ip
        if !pf.is_empty() {
            let mut candidates: Vec<&str> = vec![];
            if let Some(ref u) = cred.username {
                candidates.push(u.as_str());
            }
            for svc in &assoc_svcs {
                candidates.push(svc.name.as_str());
                if let Some(ref p) = svc.product { candidates.push(p.as_str()); }
                if let Some(addr) = all_addresses.iter().find(|a| a.id == svc.address_id) {
                    candidates.push(addr.ip.as_str());
                }
            }
            if !pf.matches(&candidates) {
                continue;
            }
        }

        let svc_ips: Vec<String> = assoc_svcs.iter()
            .filter_map(|s| all_addresses.iter().find(|a| a.id == s.address_id))
            .map(|a| a.ip.clone())
            .collect();

        match fmt {
            OutputFormat::Addresses => {
                // Credentials have no natural address; show a compact summary line.
                // If there are associated service IPs, list those as a bonus.
                let u = cred.username.as_deref().unwrap_or("<no username>");
                if svc_ips.is_empty() {
                    println!("credential: {} (id={})", u, cred.id);
                } else {
                    println!("credential: {} (id={}) @ {}", u, cred.id, svc_ips.join(", "));
                }
            }
            OutputFormat::NmapArgs => {
                let pairs: Vec<(i32, String)> = assoc_svcs.iter()
                    .filter_map(|s| {
                        all_addresses.iter().find(|a| a.id == s.address_id)
                            .map(|a| (s.port, a.ip.clone()))
                    })
                    .collect();
                if pairs.is_empty() {
                    let u = cred.username.as_deref().unwrap_or("<no username>");
                    println!("credential: {} (id={})", u, cred.id);
                } else {
                    print_nmap_args(&pairs);
                }
            }
            OutputFormat::HumanReadable => {
                let note_marker = if cred_noted.contains(&cred.id) { " *" } else { "" };
                let tags = fmt_tags(cred_tag_map.get(&cred.id));
                println!("credential: id={}{}{}", cred.id, note_marker, tags);
                if let Some(ref u) = cred.username {
                    println!("  username: {}", u);
                }
                if cred.password.is_some() {
                    println!("  password: (set)");
                }
                if cred.hash.is_some() {
                    println!("  hash:     (set)");
                }
                for svc in &assoc_svcs {
                    let ip = all_addresses.iter()
                        .find(|a| a.id == svc.address_id)
                        .map(|a| a.ip.as_str())
                        .unwrap_or("?");
                    println!("  service:  {} port={} @ {} (id={})", svc.name, svc.port, ip, svc.id);
                }
            }
            OutputFormat::Ids => {
                let u = cred.username.as_deref().unwrap_or("");
                eprintln!("credential: (username: {})", u);
                println!("{}", cred.id);
            }
        }
    }
    Ok(())
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn list_cmd(args: &Args) -> Result<(), NNError> {
    let mut conn = establish_connection(args)?;
    let Commands::List { site, tag, listing_types, filter } = &args.command else {
        return Ok(());
    };

    let fmt = OutputFormat::from_listing_types(listing_types);
    let site_id = resolve_site_id(&mut conn, site.as_deref())?;

    match filter.as_ref() {
        None | Some(ResourceTypesFilters::Host { .. }) => {
            let patterns: &[String] = match filter.as_ref() {
                Some(ResourceTypesFilters::Host { host }) => host.as_slice(),
                _ => &[],
            };
            list_hosts(&mut conn, site_id, tag, patterns, &fmt)?;
        }
        Some(ResourceTypesFilters::Address { address }) => {
            list_addresses(&mut conn, site_id, tag, address, &fmt)?;
        }
        Some(ResourceTypesFilters::Network { network }) => {
            list_networks(&mut conn, site_id, tag, network, &fmt)?;
        }
        Some(ResourceTypesFilters::Service { service, ports }) => {
            list_services(&mut conn, site_id, tag, service, ports, &fmt)?;
        }
        Some(ResourceTypesFilters::Note { note }) => {
            list_notes(&mut conn, site_id, tag, note, &fmt)?;
        }
        Some(ResourceTypesFilters::Tag { tag: tag_patterns }) => {
            list_tags(&mut conn, tag_patterns, &fmt)?;
        }
        Some(ResourceTypesFilters::Credential { credential }) => {
            list_credentials(&mut conn, tag, credential, &fmt)?;
        }
    }

    Ok(())
}
