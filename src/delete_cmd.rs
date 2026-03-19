use diesel::prelude::*;
use crate::{Args, schema::*, models::*, AnyConnection, NNError};

pub fn delete_command(args: &Args, conn: &mut AnyConnection) -> Result<(), NNError> {
    use crate::schema::*;
    use crate::models::*;
    use diesel::prelude::*;

    match &args.command {
        crate::Commands::Delete {
            ids,
            resource_type,
        } => {
            let resource_type = resource_type.to_lowercase();
            let ids: Vec<i32> = if ids.is_empty() {
                // Read all IDs from stdin
                use std::io::BufRead;
                let stdin = std::io::stdin();
                stdin
                    .lock()
                    .lines()
                    .filter_map(|l| l.ok())
                    .flat_map(|line| {
                        line.split_whitespace()
                            .filter_map(|s| s.parse::<i32>().ok())
                            .collect::<Vec<_>>()
                    })
                    .collect()
            } else {
                ids.clone()
            };
            if ids.is_empty() {
                eprintln!("No valid IDs provided.");
                return Ok(());
            }
            match resource_type.as_str() {
                "address" | "addresses" => {
                    let num_deleted = diesel::delete(address::table.filter(address::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} address(es)", num_deleted);
                }
                "host" | "hosts" => {
                    let num_deleted = diesel::delete(host::table.filter(host::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} host(s)", num_deleted);
                }
                "network" | "networks" | "net" | "nets" => {
                    let num_deleted = diesel::delete(network::table.filter(network::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} network(s)", num_deleted);
                }
                "service" | "services" => {
                    let num_deleted = diesel::delete(service::table.filter(service::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} service(s)", num_deleted);
                }
                "note" | "notes" => {
                    let num_deleted = diesel::delete(note::table.filter(note::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} note(s)", num_deleted);
                }
                "site" | "sites" => {
                    let num_deleted = diesel::delete(site::table.filter(site::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} site(s)", num_deleted);
                }
                "tag" | "tags" => {
                    let num_deleted = diesel::delete(tag::table.filter(tag::id.eq_any(ids)))
                        .execute(conn)?;
                    println!("Deleted {} tag(s)", num_deleted);
                }
                "credential" | "cred" | "creds" | "credentials" => {
                    let num_deleted = diesel::delete(
                        credential::table.filter(credential::id.eq_any(ids)),
                    )
                    .execute(conn)?;
                    println!("Deleted {} credential(s)", num_deleted);
                }
                _ => {
                    eprintln!("Unknown resource type: {}", resource_type);
                }
            
            }
        }
        _ => {
            unreachable!();
        }
    };
    Ok(())
}