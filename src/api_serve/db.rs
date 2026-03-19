use std::sync::{Arc, Mutex};
use crate::AnyConnection;

/// Thread-safe wrapper around a single database connection.
/// For a local tool a single connection under a Mutex is sufficient.
pub type DbPool = Arc<Mutex<AnyConnection>>;

pub fn new_pool(database_url: &str) -> Result<DbPool, crate::NNError> {
    let conn = if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        AnyConnection::Postgresql(diesel::PgConnection::establish(database_url)?)
    } else {
        use diesel::connection::SimpleConnection;
        let mut c = diesel::SqliteConnection::establish(database_url)?;
        c.batch_execute("PRAGMA foreign_keys = ON;")?;
        AnyConnection::Sqlite(c)
    };
    Ok(Arc::new(Mutex::new(conn)))
}
