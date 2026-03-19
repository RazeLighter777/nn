use diesel::prelude::*;
use crate::{Args, AnyConnection, NNError, models::{NewTag, NewTagAssignment}};

pub fn tag_command(args: &Args, conn: &mut AnyConnection) -> Result<(), NNError> {
    use crate::schema::{tag, tag_assignment};

    let crate::Commands::Tag { action } = &args.command else {
        unreachable!();
    };

    match action {
        crate::TagAction::Create { names } => {
            for name in names {
                let existing: Option<i32> = tag::table
                    .filter(tag::name.eq(name))
                    .select(tag::id)
                    .first(conn)
                    .optional()?;
                if let Some(id) = existing {
                    println!("Tag '{}' already exists (id={})", name, id);
                } else {
                    diesel::insert_into(tag::table)
                        .values(&NewTag { name: name.clone() })
                        .execute(conn)?;
                    let new_id: i32 = tag::table
                        .filter(tag::name.eq(name))
                        .select(tag::id)
                        .first(conn)?;
                    println!("Created tag '{}' (id={})", name, new_id);
                }
            }
        }

        crate::TagAction::Add { tag_name, resource_type, ids } => {
            // Get or create the tag
            let tag_id: i32 = {
                let existing: Option<i32> = tag::table
                    .filter(tag::name.eq(tag_name))
                    .select(tag::id)
                    .first(conn)
                    .optional()?;
                match existing {
                    Some(id) => id,
                    None => {
                        diesel::insert_into(tag::table)
                            .values(&NewTag { name: tag_name.clone() })
                            .execute(conn)?;
                        let new_id: i32 = tag::table
                            .filter(tag::name.eq(tag_name))
                            .select(tag::id)
                            .first(conn)?;
                        println!("Created tag '{}'", tag_name);
                        new_id
                    }
                }
            };

            let rt = resource_type.to_lowercase();

            if !matches!(rt.as_str(), "host" | "hosts" | "address" | "addresses"
                | "service" | "services" | "network" | "networks" | "net" | "nets"
                | "credential" | "cred" | "creds" | "credentials") {
                eprintln!("Unknown resource type: {}", resource_type);
                return Ok(());
            }

            let ids_input: Vec<i32> = if ids.is_empty() {
                use std::io::BufRead;
                std::io::stdin()
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

            for &id in &ids_input {
                // Check if already assigned
                let already: bool = {
                    let mut q = tag_assignment::table
                        .filter(tag_assignment::tag_id.eq(tag_id))
                        .into_boxed();
                    q = match rt.as_str() {
                        "host" | "hosts" =>
                            q.filter(tag_assignment::host_id.eq(Some(id))),
                        "address" | "addresses" =>
                            q.filter(tag_assignment::address_id.eq(Some(id))),
                        "service" | "services" =>
                            q.filter(tag_assignment::service_id.eq(Some(id))),
                        "credential" | "cred" | "creds" | "credentials" =>
                            q.filter(tag_assignment::credential_id.eq(Some(id))),
                        _ =>
                            q.filter(tag_assignment::network_id.eq(Some(id))),
                    };
                    q.select(tag_assignment::id).first::<i32>(conn).optional()?.is_some()
                };

                if already {
                    println!("Tag '{}' already assigned to {} {}", tag_name, rt, id);
                } else {
                    let assignment = NewTagAssignment {
                        tag_id,
                        host_id:       if matches!(rt.as_str(), "host" | "hosts") { Some(id) } else { None },
                        address_id:    if matches!(rt.as_str(), "address" | "addresses") { Some(id) } else { None },
                        service_id:    if matches!(rt.as_str(), "service" | "services") { Some(id) } else { None },
                        network_id:    if matches!(rt.as_str(), "network" | "networks" | "net" | "nets") { Some(id) } else { None },
                        credential_id: if matches!(rt.as_str(), "credential" | "cred" | "creds" | "credentials") { Some(id) } else { None },
                    };
                    diesel::insert_into(tag_assignment::table)
                        .values(&assignment)
                        .execute(conn)?;
                    println!("Tagged {} {} with '{}'", rt, id, tag_name);
                }
            }
        }
    }

    Ok(())
}
