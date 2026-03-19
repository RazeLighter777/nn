use diesel::prelude::*;
use crate::{Args, AnyConnection, NNError, models::{NewCredential, NewCredentialService}};

pub fn cred_command(args: &Args, conn: &mut AnyConnection) -> Result<(), NNError> {
    use crate::schema::{credential, credential_service};
    use crate::CredAction;

    let crate::Commands::Cred { action } = &args.command else {
        unreachable!();
    };

    match action {
        CredAction::Add { username, password, hash, service } => {
            let new_cred = NewCredential {
                username: username.clone(),
                password: password.clone(),
                hash: hash.clone(),
            };
            diesel::insert_into(credential::table)
                .values(&new_cred)
                .execute(conn)?;
            let new_id: i32 = credential::table
                .select(credential::id)
                .order(credential::id.desc())
                .first(conn)?;

            for &svc_id in service {
                diesel::insert_into(credential_service::table)
                    .values(&NewCredentialService {
                        credential_id: new_id,
                        service_id: svc_id,
                    })
                    .execute(conn)?;
            }

            println!("Created credential id={}", new_id);
            if let Some(u) = username {
                println!("  username: {}", u);
            }
            if let Some(_) = password {
                println!("  password: (set)");
            }
            if let Some(_) = hash {
                println!("  hash:     (set)");
            }
            if !service.is_empty() {
                let sids: Vec<String> = service.iter().map(|s| s.to_string()).collect();
                println!("  services: {}", sids.join(", "));
            }
        }

        CredAction::Update { id, username, password, hash, service } => {
            // Verify credential exists
            let exists: Option<i32> = credential::table
                .filter(credential::id.eq(id))
                .select(credential::id)
                .first(conn)
                .optional()?;
            if exists.is_none() {
                eprintln!("No credential with id={}", id);
                return Ok(());
            }

            // Update each field individually; empty string means set NULL
            if let Some(u) = username {
                let val: Option<&str> = if u.is_empty() { None } else { Some(u.as_str()) };
                diesel::update(credential::table.filter(credential::id.eq(id)))
                    .set(credential::username.eq(val))
                    .execute(conn)?;
            }
            if let Some(p) = password {
                let val: Option<&str> = if p.is_empty() { None } else { Some(p.as_str()) };
                diesel::update(credential::table.filter(credential::id.eq(id)))
                    .set(credential::password.eq(val))
                    .execute(conn)?;
            }
            if let Some(h) = hash {
                let val: Option<&str> = if h.is_empty() { None } else { Some(h.as_str()) };
                diesel::update(credential::table.filter(credential::id.eq(id)))
                    .set(credential::hash.eq(val))
                    .execute(conn)?;
            }

            // Replace service associations when --service flags are given
            if !service.is_empty() {
                diesel::delete(
                    credential_service::table
                        .filter(credential_service::credential_id.eq(id)),
                )
                .execute(conn)?;
                for &svc_id in service {
                    diesel::insert_into(credential_service::table)
                        .values(&NewCredentialService {
                            credential_id: *id,
                            service_id: svc_id,
                        })
                        .execute(conn)?;
                }
            }

            println!("Updated credential id={}", id);
        }
    }

    Ok(())
}
