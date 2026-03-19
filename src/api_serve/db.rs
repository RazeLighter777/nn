use std::sync::{Arc, Mutex};
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use crate::AnyConnection;

const POSTGRES_MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("migrations/postgres");
const SQLITE_MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("migrations/sqlite");

/// Thread-safe wrapper around a single database connection.
/// For a local tool a single connection under a Mutex is sufficient.
pub type DbPool = Arc<Mutex<AnyConnection>>;

pub fn new_pool(database_url: &str) -> Result<DbPool, crate::NNError> {
    let mut conn = if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        AnyConnection::Postgresql(diesel::PgConnection::establish(database_url)?)
    } else {
        use diesel::connection::SimpleConnection;
        let mut c = diesel::SqliteConnection::establish(database_url)?;
        c.batch_execute("PRAGMA foreign_keys = ON;")?;
        AnyConnection::Sqlite(c)
    };

    // Run any pending migrations automatically
    match &mut conn {
        AnyConnection::Postgresql(c) => c.run_pending_migrations(POSTGRES_MIGRATIONS)
            .map_err(|e| crate::NNError::DatabaseError(
                diesel::result::Error::QueryBuilderError(e)
            ))?,
        AnyConnection::Sqlite(c) => c.run_pending_migrations(SQLITE_MIGRATIONS)
            .map_err(|e| crate::NNError::DatabaseError(
                diesel::result::Error::QueryBuilderError(e)
            ))?,
    };

    Ok(Arc::new(Mutex::new(conn)))
}
