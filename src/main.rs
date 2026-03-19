mod models;
mod schema;
use diesel::{Queryable, Selectable};

#[derive(diesel::MultiConnection)]
pub enum AnyConnection {
    Postgresql(diesel::PgConnection),
    Sqlite(diesel::SqliteConnection),
}
fn main() {
    println!("Hello, world!");
}
