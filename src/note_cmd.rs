use diesel::prelude::*;
use crate::{Args, AnyConnection, NNError, models::NewNote};

pub fn note_command(args: &Args, conn: &mut AnyConnection) -> Result<(), NNError> {
    use crate::schema::note;

    let crate::Commands::Note { resource_type, id } = &args.command else {
        unreachable!();
    };

    let resource_type = resource_type.to_lowercase();

    // Load any existing note for this resource
    let existing: Option<crate::models::Note> = match resource_type.as_str() {
        "host" | "hosts" => note::table
            .filter(note::host_id.eq(Some(*id)))
            .first(conn)
            .optional()?,
        "address" | "addresses" => note::table
            .filter(note::address_id.eq(Some(*id)))
            .first(conn)
            .optional()?,
        "service" | "services" => note::table
            .filter(note::service_id.eq(Some(*id)))
            .first(conn)
            .optional()?,
        "network" | "networks" | "net" | "nets" => note::table
            .filter(note::network_id.eq(Some(*id)))
            .first(conn)
            .optional()?,
        _ => {
            eprintln!("Unknown resource type: {}", resource_type);
            return Ok(());
        }
    };

    let current_text = existing.as_ref().map(|n| n.text.as_str()).unwrap_or("");

    // Write current content to a temp file
    let tmppath = std::env::temp_dir().join(format!("nn2_note_{}.txt", std::process::id()));
    std::fs::write(&tmppath, current_text)?;

    // Open $EDITOR (fall back to vi)
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&tmppath)
        .status()?;

    if !status.success() {
        eprintln!("Editor exited with non-zero status; note not saved.");
        let _ = std::fs::remove_file(&tmppath);
        return Ok(());
    }

    let new_text = std::fs::read_to_string(&tmppath)?;
    let _ = std::fs::remove_file(&tmppath);

    if new_text.trim().is_empty() {
        // Delete existing note if present, otherwise do nothing
        if let Some(ref n) = existing {
            diesel::delete(note::table.filter(note::id.eq(n.id))).execute(conn)?;
            println!("Note deleted.");
        } else {
            println!("No note saved (empty).");
        }
        return Ok(());
    }

    if let Some(ref n) = existing {
        // Update
        diesel::update(note::table.filter(note::id.eq(n.id)))
            .set(note::text.eq(&new_text))
            .execute(conn)?;
        println!("Note updated.");
    } else {
        // Insert
        let rt = resource_type.as_str();
        let new_note = NewNote {
            text: new_text,
            host_id:    if matches!(rt, "host" | "hosts") { Some(*id) } else { None },
            address_id: if matches!(rt, "address" | "addresses") { Some(*id) } else { None },
            service_id: if matches!(rt, "service" | "services") { Some(*id) } else { None },
            network_id: if matches!(rt, "network" | "networks" | "net" | "nets") { Some(*id) } else { None },
        };
        diesel::insert_into(note::table).values(&new_note).execute(conn)?;
        println!("Note saved.");
    }

    Ok(())
}
