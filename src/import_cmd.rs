use diesel::prelude::*;
use std::{
    io::{self, Write},
    net::IpAddr,
};

use crate::{AnyConnection, Args, Commands, NNError, establish_connection, models, netrange, nmap_xml, schema};

// ── Data-transfer types passed to the callback ───────────────────────────────

/// An existing `address` row that matched one of the scanned host's IPs.
#[derive(Debug, Clone)]
pub struct AddressMatch {
    pub address_id: i32,
    pub host_id: i32,
    pub host_name: String,
    pub network_id: i32,
    pub network_name: String,
    pub ip: String,
    pub mac: Option<String>,
}

/// An existing host available for selection.
#[derive(Debug, Clone)]
pub struct HostOption {
    pub id: i32,
    pub name: String,
    /// IP addresses of all address rows belonging to this host.
    pub addresses: Vec<String>,
}

/// An existing network available for selection.
#[derive(Debug, Clone)]
pub struct NetworkOption {
    pub id: i32,
    pub name: String,
    /// Covering CIDR derived from this network's member addresses, e.g. `"192.168.88.0/24"`.
    /// `None` when the network has no addresses yet.
    pub covering_cidr: Option<String>,
    /// Parsed `(network_address, prefix_len)` for containment checks.
    /// `None` when the network has no addresses yet.
    pub covering_prefix: Option<(IpAddr, u8)>,
}

/// A concrete field conflict between the database and incoming scan data.
#[derive(Debug, Clone)]
pub struct ServiceConflict {
    pub field: String,
    pub existing: String,
    pub incoming: String,
}

// ── Action / Decision enums ───────────────────────────────────────────────────

/// Each variant describes a decision point during a scan import.
pub enum ProposedImportAction {
    /// The named site does not exist yet; default creates it.
    CreateNewSite { name: String },
    /// A scanned host needs matching to existing address(es) or needs new records.
    /// 0 matches → default CreateNew; 1 match → default UseExisting; N → must choose.
    MatchScannedHost {
        scanned_display_name: String,
        scanned_ips: Vec<String>,
        scanned_mac: Option<String>,
        existing_matches: Vec<AddressMatch>,
    },
    /// Choose which logical host owns new address row(s); default CreateNew.
    ChooseHostForAddress {
        scanned_display_name: String,
        suggested_name: String,
        existing_hosts: Vec<HostOption>,
    },
    /// Choose which network new address row(s) belong to; default CreateNew.
    ChooseNetworkForAddress {
        scanned_ips: Vec<String>,
        suggested_name: String,
        existing_networks: Vec<NetworkOption>,
    },
    /// Choose the prefix length (e.g. 24 for /24) for new address rows.
    ChooseNetmaskForAddress {
        scanned_ips: Vec<String>,
        /// IP-family default: 24 for IPv4, 64 for IPv6.
        default_netmask: i32,
    },
    /// An existing service has differing field values; default OverwriteConflicts.
    ResolveServiceConflict {
        protocol: String,
        port: i32,
        conflicts: Vec<ServiceConflict>,
    },
}

/// Response from the callback for any `ProposedImportAction`.
pub enum ImportScanDecision {
    /// Accept the context-specific default.
    ContinueWithDefault,
    /// Skip this item; continue importing the rest.
    Skip,
    /// Abort the entire import (triggers transaction rollback).
    Abort,
    /// Reuse an existing record by its integer primary key.
    UseExisting { id: i32 },
    /// Create a new record with the given name.
    CreateNew { name: String },
    /// Overwrite every changed service field with incoming scan values.
    OverwriteConflicts,
    /// Keep existing values; only fill NULL fields from the scan.
    KeepExisting,
    /// Use the given prefix length for new address rows.
    SetNetmask { value: i32 },
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn import_cmd(args: &Args) -> Result<(), NNError> {
    let mut conn: AnyConnection = establish_connection(args)?;
    let Commands::ImportScan { file, accept_defaults, site } = &args.command else {
        return Ok(());
    };
    if *accept_defaults {
        import_scan(&mut conn, file, site, |_action| {
            ImportScanDecision::ContinueWithDefault
        })?;
    } else {
        import_scan(&mut conn, file, site, interactive_callback)?;
    }
    Ok(())
}

// ── Interactive callback ──────────────────────────────────────────────────────

fn interactive_callback(action: ProposedImportAction) -> ImportScanDecision {
    match action {
        ProposedImportAction::CreateNewSite { name } => prompt_create_site(&name),
        ProposedImportAction::MatchScannedHost {
            scanned_display_name,
            scanned_ips,
            scanned_mac,
            existing_matches,
        } => prompt_match_host(&scanned_display_name, &scanned_ips, scanned_mac.as_deref(), &existing_matches),
        ProposedImportAction::ChooseHostForAddress {
            scanned_display_name,
            suggested_name,
            existing_hosts,
        } => prompt_choose_host(&scanned_display_name, &suggested_name, &existing_hosts),
        ProposedImportAction::ChooseNetworkForAddress {
            scanned_ips,
            suggested_name,
            existing_networks,
        } => prompt_choose_network(&scanned_ips, &suggested_name, &existing_networks),
        ProposedImportAction::ChooseNetmaskForAddress {
            scanned_ips,
            default_netmask,
        } => prompt_choose_netmask(&scanned_ips, default_netmask),
        ProposedImportAction::ResolveServiceConflict {
            protocol,
            port,
            conflicts,
        } => prompt_resolve_conflict(&protocol, port, &conflicts),
    }
}

// ── Per-action prompt functions ───────────────────────────────────────────────

fn prompt_create_site(name: &str) -> ImportScanDecision {
    println!("Site '{}' does not exist yet.", name);
    loop {
        let response = readline(&format!(
            "Create site '{}', enter a different [n]ame, [s]kip, or [a]bort? [default: create]: ",
            name
        ));
        let trimmed = response.trim().to_ascii_lowercase();
        match trimmed.as_str() {
            "" | "y" | "yes" | "create" => return ImportScanDecision::ContinueWithDefault,
            "s" | "skip" => return ImportScanDecision::Skip,
            "a" | "abort" => return ImportScanDecision::Abort,
            "n" | "name" => {
                let new_name = readline(&format!("Site name [default: {}]: ", name));
                let new_name = new_name.trim();
                let resolved = if new_name.is_empty() { name } else { new_name };
                return ImportScanDecision::CreateNew { name: resolved.to_string() };
            }
            _ => eprintln!("  Unknown option. Enter blank/y/name/s/a."),
        }
    }
}

fn prompt_match_host(
    display_name: &str,
    ips: &[String],
    mac: Option<&str>,
    existing_matches: &[AddressMatch],
) -> ImportScanDecision {
    println!();
    println!("─── Scanned host: {} ───", display_name);
    println!("    IPs : {}", if ips.is_empty() { "none".to_string() } else { ips.join(", ") });
    if let Some(m) = mac {
        println!("    MAC : {}", m);
    }

    match existing_matches.len() {
        0 => {
            // No existing matches – default is to create new.
            loop {
                let response = readline(
                    "No existing address matches. [c]reate new records, [s]kip, or [a]bort? [default: create]: ",
                );
                match response.trim().to_ascii_lowercase().as_str() {
                    "" | "c" | "create" | "new" => return ImportScanDecision::ContinueWithDefault,
                    "s" | "skip" => return ImportScanDecision::Skip,
                    "a" | "abort" => return ImportScanDecision::Abort,
                    _ => eprintln!("  Unknown option."),
                }
            }
        }

        1 => {
            // Single match – default is to merge.
            let m = &existing_matches[0];
            println!(
                "  1. Merge into existing: host='{}' network='{}' ip='{}' mac={}",
                m.host_name,
                m.network_name,
                m.ip,
                m.mac.as_deref().unwrap_or("none"),
            );
            loop {
                let response = readline(
                    "Select 1 to merge, [n]ew to create new records, [s]kip, or [a]bort [default: 1]: ",
                );
                let trimmed = response.trim().to_ascii_lowercase();
                match trimmed.as_str() {
                    "" | "1" => return ImportScanDecision::ContinueWithDefault,
                    "n" | "new" => return ImportScanDecision::CreateNew { name: String::new() },
                    "s" | "skip" => return ImportScanDecision::Skip,
                    "a" | "abort" => return ImportScanDecision::Abort,
                    _ => eprintln!("  Unknown option."),
                }
            }
        }

        _ => {
            // Multiple matches – no safe default; user must choose.
            println!("  Multiple existing addresses match this host:");
            for (i, m) in existing_matches.iter().enumerate() {
                println!(
                    "  {}. host='{}' network='{}' ip='{}' mac={}",
                    i + 1,
                    m.host_name,
                    m.network_name,
                    m.ip,
                    m.mac.as_deref().unwrap_or("none"),
                );
            }
            loop {
                let response = readline(&format!(
                    "Select 1-{} to merge, [n]ew to create new records, [s]kip, or [a]bort: ",
                    existing_matches.len()
                ));
                let trimmed = response.trim().to_ascii_lowercase();
                match trimmed.as_str() {
                    "n" | "new" => return ImportScanDecision::CreateNew { name: String::new() },
                    "s" | "skip" => return ImportScanDecision::Skip,
                    "a" | "abort" => return ImportScanDecision::Abort,
                    _ => {
                        if let Ok(n) = trimmed.parse::<usize>() {
                            if n >= 1 && n <= existing_matches.len() {
                                return ImportScanDecision::UseExisting {
                                    id: existing_matches[n - 1].address_id,
                                };
                            }
                        }
                        eprintln!("  Invalid selection.");
                    }
                }
            }
        }
    }
}

fn prompt_choose_host(
    display_name: &str,
    suggested_name: &str,
    existing_hosts: &[HostOption],
) -> ImportScanDecision {
    println!();
    println!("  Assign address to a host (scanned: '{}'):", display_name);
    if existing_hosts.is_empty() {
        println!("  (no existing hosts in this site)");
    } else {
        for (i, h) in existing_hosts.iter().enumerate() {
            let addrs = if h.addresses.is_empty() {
                "no addresses".to_string()
            } else {
                h.addresses.join(", ")
            };
            println!("  {}. {} [{}]", i + 1, h.name, addrs);
        }
    }

    let default_index = existing_hosts
        .iter()
        .position(|h| h.name.eq_ignore_ascii_case(display_name));
    let default_hint = match default_index {
        Some(i) => format!("{}", i + 1),
        None => "n".to_string(),
    };

    loop {
        let response = readline(&format!(
            "  Select 1-{}, [n]ew host, [s]kip, or [a]bort [default: {}]: ",
            existing_hosts.len().max(1),
            default_hint,
        ));
        let trimmed = response.trim().to_ascii_lowercase();
        let effective = if trimmed.is_empty() { default_hint.clone() } else { trimmed };

        match effective.as_str() {
            "n" | "new" => {
                let name_input = readline(&format!("  Host name [default: {}]: ", suggested_name));
                let name = name_input.trim();
                let resolved = if name.is_empty() { suggested_name } else { name };
                return ImportScanDecision::CreateNew { name: resolved.to_string() };
            }
            "s" | "skip" => return ImportScanDecision::Skip,
            "a" | "abort" => return ImportScanDecision::Abort,
            s => {
                if let Ok(n) = s.parse::<usize>() {
                    if n >= 1 && n <= existing_hosts.len() {
                        return ImportScanDecision::UseExisting { id: existing_hosts[n - 1].id };
                    }
                }
                eprintln!("  Invalid selection.");
            }
        }
    }
}

fn prompt_choose_network(
    ips: &[String],
    suggested_name: &str,
    existing_networks: &[NetworkOption],
) -> ImportScanDecision {
    println!();
    println!("  Assign address(es) [{}] to a network:", ips.join(", "));
    if existing_networks.is_empty() {
        println!("  (no existing networks in this site)");
    } else {
        for (i, n) in existing_networks.iter().enumerate() {
            // Show covering CIDR in brackets when it differs from the stored name
            // (avoids the "192.168.88.0/24 [192.168.88.0/24]" duplicate look).
            let suffix = match &n.covering_cidr {
                Some(cidr) if cidr != &n.name => format!(" [covers {}]", cidr),
                Some(_) => String::new(),
                None => " (no addresses yet)".to_string(),
            };
            println!("  {}. {}{}", i + 1, n.name, suffix);
        }
    }

    // Determine the best default: the most-specific existing network whose
    // covering prefix contains *all* of the incoming IPs.  Fall back to
    // "create new" when nothing matches.
    let default_hint = {
        let parsed_incoming: Vec<IpAddr> = ips
            .iter()
            .filter_map(|s| s.parse::<IpAddr>().ok())
            .collect();

        // Keep only networks that have a computed covering prefix, together
        // with their original slice index so we can map back after filtering.
        let indexed: Vec<(usize, (IpAddr, u8))> = existing_networks
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.covering_prefix.map(|p| (i, p)))
            .collect();

        let best = indexed
            .iter()
            .filter(|(_, (net_addr, prefix_len))| {
                !parsed_incoming.is_empty()
                    && parsed_incoming
                        .iter()
                        .all(|ip| netrange::ip_in_prefix(*ip, *net_addr, *prefix_len))
            })
            .max_by_key(|(_, (_, prefix_len))| *prefix_len)
            .map(|(orig_i, _)| *orig_i);

        match best {
            Some(i) => format!("{}", i + 1),
            None => "c".to_string(),
        }
    };

    loop {
        let response = readline(&format!(
            "  Select 1-{}, [c]reate new network, [s]kip, or [a]bort [default: {}]: ",
            existing_networks.len().max(1),
            default_hint,
        ));
        let trimmed = response.trim().to_ascii_lowercase();
        let effective = if trimmed.is_empty() { default_hint.clone() } else { trimmed };

        match effective.as_str() {
            "c" | "create" | "n" | "new" => {
                let name_input = readline(&format!("  Network name [default: {}]: ", suggested_name));
                let name = name_input.trim();
                let resolved = if name.is_empty() { suggested_name } else { name };
                return ImportScanDecision::CreateNew { name: resolved.to_string() };
            }
            "s" | "skip" => return ImportScanDecision::Skip,
            "a" | "abort" => return ImportScanDecision::Abort,
            s => {
                if let Ok(n) = s.parse::<usize>() {
                    if n >= 1 && n <= existing_networks.len() {
                        return ImportScanDecision::UseExisting { id: existing_networks[n - 1].id };
                    }
                }
                eprintln!("  Invalid selection.");
            }
        }
    }
}

fn prompt_choose_netmask(ips: &[String], default_netmask: i32) -> ImportScanDecision {
    println!();
    let response = readline(&format!(
        "  Prefix length for {} [default: /{}]: ",
        ips.join(", "),
        default_netmask,
    ));
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return ImportScanDecision::ContinueWithDefault;
    }
    // Accept both "24" and "/24" formats.
    let cleaned = trimmed.trim_start_matches('/');
    match cleaned.parse::<i32>() {
        Ok(n) if n >= 0 && n <= 128 => ImportScanDecision::SetNetmask { value: n },
        _ => {
            eprintln!("  Invalid prefix length; using default /{}", default_netmask);
            ImportScanDecision::ContinueWithDefault
        }
    }
}

fn prompt_resolve_conflict(
    protocol: &str,
    port: i32,
    conflicts: &[ServiceConflict],
) -> ImportScanDecision {
    println!();
    println!("  Service {}/{} has changed values:", protocol, port);
    for c in conflicts {
        println!("    {}: '{}' → '{}'", c.field, c.existing, c.incoming);
    }
    loop {
        let response = readline(
            "  [o]verwrite with new values, [k]eep existing, [s]kip service, or [a]bort [default: overwrite]: ",
        );
        match response.trim().to_ascii_lowercase().as_str() {
            "" | "o" | "overwrite" | "y" => return ImportScanDecision::OverwriteConflicts,
            "k" | "keep" => return ImportScanDecision::KeepExisting,
            "s" | "skip" => return ImportScanDecision::Skip,
            "a" | "abort" => return ImportScanDecision::Abort,
            _ => eprintln!("  Unknown option."),
        }
    }
}

// ── stdin helper ──────────────────────────────────────────────────────────────

fn readline(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
    buf.trim_end_matches('\n').trim_end_matches('\r').to_string()
}

// ── Core import logic ─────────────────────────────────────────────────────────

pub fn import_scan(
    conn: &mut AnyConnection,
    file: &str,
    site_name: &str,
    callback: impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<(), NNError> {
    let xml_data = std::fs::read_to_string(file)?;
    let nmap_run: nmap_xml::NmapRunXml = nmap_xml::parse_nmap_xml(&xml_data)?;

    conn.transaction::<(), NNError, _>(|t| {
        let site_id = match resolve_site(t, site_name, &callback)? {
            Some(id) => id,
            None => return Ok(()),
        };
        for host_xml in &nmap_run.hosts {
            import_host(t, site_id, host_xml, &callback)?;
        }
        Ok(())
    })?;

    Ok(())
}

// ── Site resolution ───────────────────────────────────────────────────────────

fn resolve_site(
    conn: &mut AnyConnection,
    site_name: &str,
    callback: &impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<Option<i32>, NNError> {
    let existing: Vec<models::Site> = schema::site::table
        .filter(schema::site::name.eq(site_name))
        .load(conn)?;

    if let Some(site) = existing.into_iter().next() {
        return Ok(Some(site.id));
    }

    let decision = callback(ProposedImportAction::CreateNewSite {
        name: site_name.to_string(),
    });

    let name = match decision {
        ImportScanDecision::ContinueWithDefault => site_name.to_string(),
        ImportScanDecision::CreateNew { name } => name,
        ImportScanDecision::Skip => return Ok(None),
        ImportScanDecision::Abort => return Err(NNError::Aborted),
        _ => site_name.to_string(),
    };

    diesel::insert_into(schema::site::table)
        .values(schema::site::name.eq(&name))
        .execute(conn)?;

    let id: i32 = schema::site::table
        .order(schema::site::id.desc())
        .select(schema::site::id)
        .first(conn)?;

    Ok(Some(id))
}

// ── Host import ───────────────────────────────────────────────────────────────

fn import_host(
    conn: &mut AnyConnection,
    site_id: i32,
    host_xml: &nmap_xml::HostXml,
    callback: &impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<(), NNError> {
    let scanned_ips = host_xml.ip_addresses();
    let scanned_mac = host_xml.mac_address();
    let display_name = host_xml.display_name();

    let existing_matches = find_matching_addresses(conn, &scanned_ips)?;

    match existing_matches.len() {
        0 => {
            let decision = callback(ProposedImportAction::MatchScannedHost {
                scanned_display_name: display_name.clone(),
                scanned_ips: scanned_ips.clone(),
                scanned_mac: scanned_mac.clone(),
                existing_matches: vec![],
            });
            match decision {
                ImportScanDecision::Skip => {}
                ImportScanDecision::Abort => return Err(NNError::Aborted),
                // Default and CreateNew both create new records.
                _ => attach_new_addresses(conn, site_id, host_xml, callback)?,
            }
        }

        1 => {
            let default_id = existing_matches[0].address_id;
            let decision = callback(ProposedImportAction::MatchScannedHost {
                scanned_display_name: display_name.clone(),
                scanned_ips: scanned_ips.clone(),
                scanned_mac: scanned_mac.clone(),
                existing_matches: existing_matches.clone(),
            });
            match decision {
                ImportScanDecision::ContinueWithDefault => {
                    merge_into_address(conn, site_id, default_id, host_xml, callback)?;
                }
                ImportScanDecision::UseExisting { id } => {
                    merge_into_address(conn, site_id, id, host_xml, callback)?;
                }
                ImportScanDecision::CreateNew { .. } => {
                    attach_new_addresses(conn, site_id, host_xml, callback)?;
                }
                ImportScanDecision::Skip => {}
                ImportScanDecision::Abort => return Err(NNError::Aborted),
                _ => merge_into_address(conn, site_id, default_id, host_xml, callback)?,
            }
        }

        _ => {
            let decision = callback(ProposedImportAction::MatchScannedHost {
                scanned_display_name: display_name.clone(),
                scanned_ips: scanned_ips.clone(),
                scanned_mac: scanned_mac.clone(),
                existing_matches: existing_matches.clone(),
            });
            match decision {
                ImportScanDecision::UseExisting { id } => {
                    merge_into_address(conn, site_id, id, host_xml, callback)?;
                }
                ImportScanDecision::CreateNew { .. } => {
                    attach_new_addresses(conn, site_id, host_xml, callback)?;
                }
                ImportScanDecision::Abort => return Err(NNError::Aborted),
                // Skip or ContinueWithDefault (no safe default) → skip.
                _ => {}
            }
        }
    }

    Ok(())
}

// ── Merge scan data into an existing address row ──────────────────────────────

fn merge_into_address(
    conn: &mut AnyConnection,
    site_id: i32,
    address_id: i32,
    host_xml: &nmap_xml::HostXml,
    callback: &impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<(), NNError> {
    // Update MAC if not yet recorded.
    if let Some(mac) = host_xml.mac_address() {
        diesel::update(schema::address::table.find(address_id))
            .filter(schema::address::mac.is_null())
            .set(schema::address::mac.eq(&mac))
            .execute(conn)?;
    }

    let host_id: i32 = schema::address::table
        .find(address_id)
        .select(schema::address::host_id)
        .first(conn)?;

    // Fill in OS type and hostname if not already stored.
    if let Some(os) = host_xml
        .os
        .as_ref()
        .and_then(|os| os.osmatch.first())
        .map(|m| m.name.as_str())
    {
        diesel::update(schema::host::table.find(host_id))
            .filter(schema::host::os_type.is_null())
            .set(schema::host::os_type.eq(os))
            .execute(conn)?;
    }
    if let Some(hn) = host_xml.hostname_values().into_iter().next() {
        diesel::update(schema::host::table.find(host_id))
            .filter(schema::host::hostname.is_null())
            .set(schema::host::hostname.eq(&hn))
            .execute(conn)?;
    }

    upsert_services(conn, site_id, address_id, host_xml, callback)
}

// ── Create new host + network + address rows ──────────────────────────────────

fn attach_new_addresses(
    conn: &mut AnyConnection,
    site_id: i32,
    host_xml: &nmap_xml::HostXml,
    callback: &impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<(), NNError> {
    let ips = host_xml.ip_addresses();
    let display_name = host_xml.display_name();

    // ── Choose / create host ─────────────────────────────────────────────────
    let existing_hosts = load_host_options(conn, site_id)?;
    let decision = callback(ProposedImportAction::ChooseHostForAddress {
        scanned_display_name: display_name.clone(),
        suggested_name: display_name.clone(),
        existing_hosts,
    });

    let host_id = match decision {
        ImportScanDecision::ContinueWithDefault => {
            create_host(conn, site_id, &display_name, host_xml)?
        }
        ImportScanDecision::CreateNew { name } => create_host(conn, site_id, &name, host_xml)?,
        ImportScanDecision::UseExisting { id } => id,
        ImportScanDecision::Skip => return Ok(()),
        ImportScanDecision::Abort => return Err(NNError::Aborted),
        _ => create_host(conn, site_id, &display_name, host_xml)?,
    };

    // ── Choose / create network ──────────────────────────────────────────────
    let existing_networks = load_network_options(conn, site_id)?;
    let suggested_net = suggested_network_name(&ips);
    let net_decision = callback(ProposedImportAction::ChooseNetworkForAddress {
        scanned_ips: ips.clone(),
        suggested_name: suggested_net.clone(),
        // Clone so we still own `existing_networks` for the netmask default below.
        existing_networks: existing_networks.clone(),
    });

    // Extract the network id and remember which covering prefix (if any) the
    // chosen network has — used to offer an intelligent netmask default.
    let (network_id, chosen_net_prefix) = match net_decision {
        ImportScanDecision::ContinueWithDefault => {
            // Reuse an existing network whose covering prefix contains all
            // incoming IPs; only create a new one when nothing matches.
            let parsed_ips: Vec<std::net::IpAddr> = ips.iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            let nets_with_prefix: Vec<&NetworkOption> = existing_networks.iter()
                .filter(|n| n.covering_prefix.is_some())
                .collect();
            let candidates: Vec<(std::net::IpAddr, u8)> = nets_with_prefix.iter()
                .map(|n| n.covering_prefix.unwrap())
                .collect();
            if let Some(idx) = netrange::best_matching_network(&candidates, &parsed_ips) {
                let matched = nets_with_prefix[idx];
                (matched.id, matched.covering_prefix)
            } else {
                (create_network(conn, site_id, &suggested_net)?, None)
            }
        }
        ImportScanDecision::CreateNew { name } => {
            (create_network(conn, site_id, &name)?, None)
        }
        ImportScanDecision::UseExisting { id } => {
            let prefix = existing_networks.iter()
                .find(|n| n.id == id)
                .and_then(|n| n.covering_prefix);
            (id, prefix)
        }
        ImportScanDecision::Skip => return Ok(()),
        ImportScanDecision::Abort => return Err(NNError::Aborted),
        _ => (create_network(conn, site_id, &suggested_net)?, None),
    };

    // ── Choose prefix length for new address rows ────────────────────────────
    // Default: the chosen network's covering prefix_len, or the IP-family default.
    let ip_family_default = ips.first()
        .map(|s| ip_family_and_default_netmask(s).1)
        .unwrap_or(24);
    let default_netmask = chosen_net_prefix
        .map(|(_, plen)| plen as i32)
        .unwrap_or(ip_family_default);
    let netmask_decision = callback(ProposedImportAction::ChooseNetmaskForAddress {
        scanned_ips: ips.clone(),
        default_netmask,
    });
    let chosen_netmask = match netmask_decision {
        ImportScanDecision::ContinueWithDefault => default_netmask,
        ImportScanDecision::SetNetmask { value } => value,
        ImportScanDecision::Skip => return Ok(()),
        ImportScanDecision::Abort => return Err(NNError::Aborted),
        _ => default_netmask,
    };

    // ── Insert one address row per IP ────────────────────────────────────────
    let mac = host_xml.mac_address();
    for ip_str in &ips {
        let (family, _) = ip_family_and_default_netmask(ip_str);
        diesel::insert_into(schema::address::table)
            .values((
                schema::address::host_id.eq(host_id),
                schema::address::network_id.eq(network_id),
                schema::address::ip.eq(ip_str),
                schema::address::ip_family.eq(family),
                schema::address::netmask.eq(chosen_netmask),
                schema::address::mac.eq(mac.as_deref()),
            ))
            .execute(conn)?;
    }

    // Upsert services against the first IP's address row.
    if let Some(first_ip) = ips.first() {
        let address_id: i32 = schema::address::table
            .filter(schema::address::host_id.eq(host_id))
            .filter(schema::address::ip.eq(first_ip))
            .select(schema::address::id)
            .first(conn)?;

        upsert_services(conn, site_id, address_id, host_xml, callback)?;
    }

    Ok(())
}

// ── Service upsert ────────────────────────────────────────────────────────────

fn upsert_services(
    conn: &mut AnyConnection,
    site_id: i32,
    address_id: i32,
    host_xml: &nmap_xml::HostXml,
    callback: &impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<(), NNError> {
    let Some(ports) = host_xml.ports.as_ref() else {
        return Ok(());
    };
    for port in &ports.port {
        upsert_service_for_port(conn, site_id, address_id, port, callback)?;
    }
    Ok(())
}

fn upsert_service_for_port(
    conn: &mut AnyConnection,
    site_id: i32,
    address_id: i32,
    port: &nmap_xml::PortXml,
    callback: &impl Fn(ProposedImportAction) -> ImportScanDecision,
) -> Result<(), NNError> {
    let protocol = &port.protocol;
    let portid = i32::from(port.portid);
    let ip_proto = protocol_to_number(protocol);

    let state = port
        .state
        .as_ref()
        .and_then(|s| s.state.as_deref())
        .unwrap_or("unknown")
        .to_string();

    let svc = port.service.as_ref();
    let name = svc.and_then(|s| s.name.as_deref()).unwrap_or("").to_string();
    let product = svc.and_then(|s| s.product.as_deref());
    let version = svc.and_then(|s| s.version.as_deref());
    let extra_info = svc.and_then(|s| s.extrainfo.as_deref());
    let os_type = svc.and_then(|s| s.ostype.as_deref());
    let device_type = svc.and_then(|s| s.devicetype.as_deref());
    let hostname = svc.and_then(|s| s.hostname.as_deref());
    let confidence = svc.and_then(|s| s.conf);
    let method = svc.and_then(|s| s.method.as_deref());
    let service_fp = svc.and_then(|s| s.servicefp.as_deref());
    let cpe_str: Option<String> = svc
        .map(|s| {
            s.cpe
                .iter()
                .filter_map(|c| c.value.as_deref())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|s| !s.is_empty());
    let rpcnum = svc.and_then(|s| s.rpcnum);
    let lowver = svc.and_then(|s| s.lowver);
    let highver = svc.and_then(|s| s.highver);
    let owner = port.owner.as_ref().and_then(|o| o.name.as_deref());

    let existing: Option<models::Service> = schema::service::table
        .filter(schema::service::address_id.eq(address_id))
        .filter(schema::service::port.eq(portid))
        .filter(schema::service::ip_proto_number.eq(ip_proto))
        .select(models::Service::as_select())
        .first(conn)
        .optional()?;

    if let Some(ex) = existing {
        // Collect fields that differ (both old and new must be non-empty).
        let mut conflicts: Vec<ServiceConflict> = Vec::new();

        macro_rules! check_str {
            ($ex_field:expr, $new_val:expr, $label:literal) => {
                if let Some(new_val) = $new_val {
                    let old = $ex_field.as_deref().unwrap_or("");
                    if !old.is_empty() && !old.eq_ignore_ascii_case(new_val) {
                        conflicts.push(ServiceConflict {
                            field: $label.to_string(),
                            existing: old.to_string(),
                            incoming: new_val.to_string(),
                        });
                    }
                }
            };
        }

        if !name.is_empty() && !ex.name.is_empty() && !ex.name.eq_ignore_ascii_case(&name) {
            conflicts.push(ServiceConflict {
                field: "name".to_string(),
                existing: ex.name.clone(),
                incoming: name.clone(),
            });
        }
        check_str!(ex.product, product, "product");
        check_str!(ex.version, version, "version");
        check_str!(ex.extra_info, extra_info, "extra_info");
        check_str!(ex.os_type, os_type, "os_type");
        check_str!(ex.device_type, device_type, "device_type");
        check_str!(ex.hostname, hostname, "hostname");
        check_str!(ex.method, method, "method");
        check_str!(ex.service_fp, service_fp, "service_fp");
        check_str!(ex.cpe, cpe_str.as_deref(), "cpe");
        check_str!(ex.owner, owner, "owner");

        let overwrite = if conflicts.is_empty() {
            true
        } else {
            let decision = callback(ProposedImportAction::ResolveServiceConflict {
                protocol: protocol.clone(),
                port: portid,
                conflicts,
            });
            match decision {
                ImportScanDecision::ContinueWithDefault
                | ImportScanDecision::OverwriteConflicts => true,
                ImportScanDecision::KeepExisting => false,
                ImportScanDecision::Skip => return Ok(()),
                ImportScanDecision::Abort => return Err(NNError::Aborted),
                _ => true,
            }
        };

        // Always update state.
        diesel::update(schema::service::table.find(ex.id))
            .set(schema::service::state.eq(&state))
            .execute(conn)?;

        if overwrite {
            if !name.is_empty() {
                diesel::update(schema::service::table.find(ex.id))
                    .set(schema::service::name.eq(&name))
                    .execute(conn)?;
            }
            macro_rules! upd {
                ($col:expr, $val:expr) => {
                    if let Some(v) = $val {
                        diesel::update(schema::service::table.find(ex.id))
                            .set($col.eq(v))
                            .execute(conn)?;
                    }
                };
            }
            upd!(schema::service::product, product);
            upd!(schema::service::version, version);
            upd!(schema::service::extra_info, extra_info);
            upd!(schema::service::os_type, os_type);
            upd!(schema::service::device_type, device_type);
            upd!(schema::service::hostname, hostname);
            upd!(schema::service::confidence, confidence);
            upd!(schema::service::method, method);
            upd!(schema::service::service_fp, service_fp);
            upd!(schema::service::cpe, cpe_str.as_deref());
            upd!(schema::service::rpcnum, rpcnum);
            upd!(schema::service::lowver, lowver);
            upd!(schema::service::highver, highver);
            upd!(schema::service::owner, owner);
        } else {
            // Fill-NULL mode.
            if !name.is_empty() && ex.name.is_empty() {
                diesel::update(schema::service::table.find(ex.id))
                    .set(schema::service::name.eq(&name))
                    .execute(conn)?;
            }
            macro_rules! upd_null {
                ($col:expr, $existing:expr, $val:expr) => {
                    if $existing.is_none() {
                        if let Some(v) = $val {
                            diesel::update(schema::service::table.find(ex.id))
                                .set($col.eq(v))
                                .execute(conn)?;
                        }
                    }
                };
            }
            upd_null!(schema::service::product, ex.product, product);
            upd_null!(schema::service::version, ex.version, version);
            upd_null!(schema::service::extra_info, ex.extra_info, extra_info);
            upd_null!(schema::service::os_type, ex.os_type, os_type);
            upd_null!(schema::service::device_type, ex.device_type, device_type);
            upd_null!(schema::service::hostname, ex.hostname, hostname);
            upd_null!(schema::service::confidence, ex.confidence, confidence);
            upd_null!(schema::service::method, ex.method, method);
            upd_null!(schema::service::service_fp, ex.service_fp, service_fp);
            upd_null!(schema::service::cpe, ex.cpe, cpe_str.as_deref());
            upd_null!(schema::service::rpcnum, ex.rpcnum, rpcnum);
            upd_null!(schema::service::lowver, ex.lowver, lowver);
            upd_null!(schema::service::highver, ex.highver, highver);
            upd_null!(schema::service::owner, ex.owner, owner);
        }
    } else {
        diesel::insert_into(schema::service::table)
            .values((
                schema::service::site_id.eq(site_id),
                schema::service::address_id.eq(address_id),
                schema::service::port.eq(portid),
                schema::service::ip_proto_number.eq(ip_proto),
                schema::service::state.eq(&state),
                schema::service::name.eq(&name),
                schema::service::product.eq(product),
                schema::service::version.eq(version),
                schema::service::extra_info.eq(extra_info),
                schema::service::os_type.eq(os_type),
                schema::service::device_type.eq(device_type),
                schema::service::hostname.eq(hostname),
                schema::service::confidence.eq(confidence),
                schema::service::method.eq(method),
                schema::service::service_fp.eq(service_fp),
                schema::service::cpe.eq(cpe_str.as_deref()),
                schema::service::rpcnum.eq(rpcnum),
                schema::service::lowver.eq(lowver),
                schema::service::highver.eq(highver),
                schema::service::owner.eq(owner),
            ))
            .execute(conn)?;
    }

    Ok(())
}

// ── Query helpers ─────────────────────────────────────────────────────────────

fn find_matching_addresses(
    conn: &mut AnyConnection,
    ips: &[String],
) -> Result<Vec<AddressMatch>, NNError> {
    if ips.is_empty() {
        return Ok(Vec::new());
    }

    let mut matches: Vec<AddressMatch> = Vec::new();
    for ip in ips {
        let rows: Vec<(i32, i32, String, i32, String, Option<String>)> = schema::address::table
            .inner_join(schema::host::table)
            .inner_join(schema::network::table)
            .filter(schema::address::ip.eq(ip))
            .select((
                schema::address::id,
                schema::host::id,
                schema::host::name,
                schema::network::id,
                schema::network::name,
                schema::address::mac,
            ))
            .load(conn)?;

        for (addr_id, host_id, host_name, net_id, net_name, mac) in rows {
            if !matches.iter().any(|m| m.address_id == addr_id) {
                matches.push(AddressMatch {
                    address_id: addr_id,
                    host_id,
                    host_name,
                    network_id: net_id,
                    network_name: net_name,
                    ip: ip.clone(),
                    mac,
                });
            }
        }
    }
    Ok(matches)
}

fn load_host_options(conn: &mut AnyConnection, site_id: i32) -> Result<Vec<HostOption>, NNError> {
    let hosts: Vec<(i32, String)> = schema::host::table
        .filter(schema::host::site_id.eq(site_id))
        .select((schema::host::id, schema::host::name))
        .order(schema::host::name.asc())
        .load(conn)?;

    let mut options = Vec::with_capacity(hosts.len());
    for (id, name) in hosts {
        let addresses: Vec<String> = schema::address::table
            .filter(schema::address::host_id.eq(id))
            .select(schema::address::ip)
            .load(conn)?;
        options.push(HostOption { id, name, addresses });
    }
    Ok(options)
}

fn load_network_options(
    conn: &mut AnyConnection,
    site_id: i32,
) -> Result<Vec<NetworkOption>, NNError> {
    // Load all networks in this site, ordered by name.
    let networks: Vec<(i32, String)> = schema::network::table
        .filter(schema::network::site_id.eq(site_id))
        .select((schema::network::id, schema::network::name))
        .order(schema::network::name.asc())
        .load(conn)?;

    // Load all (network_id, ip, netmask) triples for addresses in these networks
    // in a single query, then group them in Rust.
    let all_addrs: Vec<(i32, String, i32)> = schema::address::table
        .inner_join(schema::network::table)
        .filter(schema::network::site_id.eq(site_id))
        .select((schema::address::network_id, schema::address::ip, schema::address::netmask))
        .load(conn)?;

    // Group (IpAddr, prefix_len) pairs by network_id.
    use std::collections::HashMap;
    let mut ips_by_network: HashMap<i32, Vec<(IpAddr, u8)>> = HashMap::new();
    for (nid, ip_str, netmask) in all_addrs {
        if let Ok(addr) = ip_str.parse::<IpAddr>() {
            let prefix_len = u8::try_from(netmask).unwrap_or(24);
            ips_by_network.entry(nid).or_default().push((addr, prefix_len));
        }
    }

    // Build NetworkOption using subnet-aware covering prefix computation.
    Ok(networks
        .into_iter()
        .map(|(id, name)| {
            let covering_prefix = ips_by_network
                .get(&id)
                .and_then(|ips| netrange::covering_cidr_with_masks(ips));

            let covering_cidr = covering_prefix
                .map(|(net_addr, prefix_len)| netrange::format_cidr(net_addr, prefix_len));

            NetworkOption { id, name, covering_cidr, covering_prefix }
        })
        .collect())
}

// ── Insert helpers ────────────────────────────────────────────────────────────

fn create_host(
    conn: &mut AnyConnection,
    site_id: i32,
    name: &str,
    host_xml: &nmap_xml::HostXml,
) -> Result<i32, NNError> {
    let os_type = host_xml
        .os
        .as_ref()
        .and_then(|os| os.osmatch.first())
        .map(|m| m.name.as_str());
    let hostname = host_xml.hostname_values().into_iter().next();

    diesel::insert_into(schema::host::table)
        .values((
            schema::host::site_id.eq(site_id),
            schema::host::name.eq(name),
            schema::host::os_type.eq(os_type),
            schema::host::hostname.eq(hostname.as_deref()),
        ))
        .execute(conn)?;

    let id: i32 = schema::host::table
        .filter(schema::host::site_id.eq(site_id))
        .filter(schema::host::name.eq(name))
        .select(schema::host::id)
        .order(schema::host::id.desc())
        .first(conn)?;

    Ok(id)
}

fn create_network(conn: &mut AnyConnection, site_id: i32, name: &str) -> Result<i32, NNError> {
    diesel::insert_into(schema::network::table)
        .values((
            schema::network::site_id.eq(site_id),
            schema::network::name.eq(name),
        ))
        .execute(conn)?;

    let id: i32 = schema::network::table
        .filter(schema::network::site_id.eq(site_id))
        .filter(schema::network::name.eq(name))
        .select(schema::network::id)
        .order(schema::network::id.desc())
        .first(conn)?;

    Ok(id)
}

// ── Utility functions ─────────────────────────────────────────────────────────

fn protocol_to_number(protocol: &str) -> i32 {
    match protocol.to_ascii_lowercase().as_str() {
        "tcp" => 6,
        "udp" => 17,
        "sctp" => 132,
        "icmp" => 1,
        "gre" => 47,
        "esp" => 50,
        "ah" => 51,
        _ => 0,
    }
}

fn ip_family_and_default_netmask(ip: &str) -> (i32, i32) {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) => (4, 24),
        Ok(IpAddr::V6(_)) => (6, 64),
        Err(_) => (4, 24),
    }
}

fn suggested_network_name(ips: &[String]) -> String {
    if let Some(ip_str) = ips.first() {
        if let Ok(IpAddr::V4(v4)) = ip_str.parse::<IpAddr>() {
            let o = v4.octets();
            return format!("{}.{}.{}.0/24", o[0], o[1], o[2]);
        }
        if let Ok(IpAddr::V6(_)) = ip_str.parse::<IpAddr>() {
            return format!("{}/64", ip_str);
        }
    }
    "imported-network".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_scan_parses() {
        let xml = std::fs::read_to_string("test/home.xml").expect("read test/home.xml");
        let dat = nmap_xml::parse_nmap_xml(&xml).expect("parse XML");
        println!("hosts={}", dat.hosts.len());
        assert!(!dat.hosts.is_empty());
    }

    #[test]
    fn test_protocol_to_number() {
        assert_eq!(protocol_to_number("tcp"), 6);
        assert_eq!(protocol_to_number("udp"), 17);
        assert_eq!(protocol_to_number("sctp"), 132);
    }

    #[test]
    fn test_ip_family() {
        assert_eq!(ip_family_and_default_netmask("192.168.1.1"), (4, 24));
        assert_eq!(ip_family_and_default_netmask("::1"), (6, 64));
    }

    #[test]
    fn test_suggested_network_name() {
        assert_eq!(
            suggested_network_name(&["192.168.1.5".to_string()]),
            "192.168.1.0/24"
        );
    }
}
