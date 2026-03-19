use diesel::prelude::*;
use thiserror::Error;
use clap::Parser;
mod models;
mod schema;
mod nmap_xml;
mod netrange;
mod import_cmd;
mod list_cmd;
mod delete_cmd;
mod note_cmd;
mod tag_cmd;
mod cred_cmd;

#[derive(diesel::MultiConnection)]
pub enum AnyConnection {
    Postgresql(diesel::PgConnection),
    Sqlite(diesel::SqliteConnection),
}

#[derive(Error, Debug)]
pub enum NNError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Connection error: {0}")]
    ConnectionError(#[from] diesel::ConnectionError),
    #[error("XML parsing error: {0}")]
    XmlParsingError(#[from] quick_xml::DeError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Import aborted by user")]
    Aborted,
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),
}

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[clap(short, long, help = "Database URL to connect to", default_value = "sqlite://database.db")]
    pub database_url: Option<String>,
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum ResourceTypesFilters {
    Host {
        #[clap(help = "Matches host+ ip/cidr with regex, e.g. web-server or 192.168")]
        host: Vec<String>,
    },
    Address {
        #[clap(help = "Matches address ip with regex, e.g. 192.168")]
        address: Vec<String>,
    },
    #[clap(alias = "net")]
    Network {
        #[clap(help = "Matches network name + ip/cidr with regex, e.g. office or 192.168")]
        network: Vec<String>,
    },
    Service {
        #[clap(help = "Matches service name + product + version with regex, e.g. http or ssl or apache")]
        service: Vec<String>,
        #[clap(long, short, help = "Filter services by port number(s), e.g. 80 or 80,443")]
        ports: Vec<String>,
    },
    Note {
        #[clap(help = "Matches note text with regex, e.g. critical or needs patching")]
        note: Vec<String>,
    },
    Tag {
        #[clap(help = "Matches tag name with regex, e.g. web or prod")]
        tag: Vec<String>,
    },
    #[clap(alias = "cred", alias = "creds")]
    Credential {
        #[clap(help = "Matches credential username or associated service name with regex")]
        credential: Vec<String>,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum TagAction {
    #[clap(name = "add", about = "Assign a tag to resource(s) — creates the tag if it doesn't exist")]
    Add {
        #[clap(help = "Tag name")]
        tag_name: String,
        #[clap(help = "Resource type: host, address, service, network")]
        resource_type: String,
        #[clap(help = "Resource ID(s). Reads from stdin if omitted.")]
        ids: Vec<i32>,
    },
    #[clap(name = "create", about = "Create new tag(s) without assigning them")]
    Create {
        #[clap(help = "Tag name(s) to create")]
        names: Vec<String>,
    },
}


#[derive(clap::Subcommand, Debug)]
pub enum CredAction {
    #[clap(name = "add", about = "Add a new credential and associate it with service(s)")]
    Add {
        #[clap(long, help = "Username")]
        username: Option<String>,
        #[clap(long, help = "Password")]
        password: Option<String>,
        #[clap(long, help = "Password hash")]
        hash: Option<String>,
        #[clap(long, short, help = "Service ID(s) to associate this credential with")]
        service: Vec<i32>,
    },
    #[clap(name = "update", about = "Update an existing credential")]
    Update {
        #[clap(help = "Credential ID to update")]
        id: i32,
        #[clap(long, help = "New username (empty string clears the field)")]
        username: Option<String>,
        #[clap(long, help = "New password (empty string clears the field)")]
        password: Option<String>,
        #[clap(long, help = "New hash (empty string clears the field)")]
        hash: Option<String>,
        #[clap(long, short, help = "Replace associated service ID(s)")]
        service: Vec<i32>,
    },
}

#[derive(clap::Args, Debug)]
#[group(required = false, multiple = false)]
pub struct ListingTypes {
    /// List just ip addresses
    /// For hosts, lists all addresses. 
    /// For networks, lists CIDR ranges.
    /// For services, lists the ip addresses of the associated services.
    /// No output is produced if the resource(s) have no associated addresses.
    #[clap(short,long, help = r"
    Output addresses for the resource(s). 
    
    (Suitable for nmap host lists)

    For hosts, lists all addresses on all nics.
    For addresses, just lists the addresses.
    For networks, lists CIDR ranges. 
    For services, lists the ip addresses of the associated services.
    For sites, lists all addresses of all hosts in the site.
    
    No output is produced if the resource(s) have no associated addresses.
    ", default_value_t = true, global=true)]
    addresses: bool,
    // give output you can paste into nmap args to target the resource(s)
    #[clap(short,long, help = r"
    Output nmap target arguments for the resource(s).
    Will be in the form of -p <ports> <targets>

    For hosts, lists all addresses on all nics, and -p <ports> for all services on the host's nics
    For addresses, just lists the addresses and -p <ports> for all services on the addresses
    For networks, lists CIDR ranges, and -p <ports> for all services on addresses in the network
    For services, lists the ip addresses of the associated services, and -p <ports> for the service's port
    For sites, lists all addresses of all hosts in the site, and -p <ports> for all services on those hosts.

    No output is produced if the resource(s) have no associated addresses or services.
    ", default_value_t = false, global=true)]
    nmap_args: bool,
    // give human readable output of the resource(s) with associated addresses and services
    #[clap(short,long, help = r"
    Output human readable information about the resource(s), including associated addresses and services.
    Will print each field of the resources, using deserialization.

    Works for all resource types."
    , default_value_t = false, global=true)]
    readable: bool,
    #[clap(short,long, help = r"
    Output in a format suitable for scripting, e.g. just ID numbers or IP addresses, one per line.
    Additional details may be printed to stderr for context, but the main output seen by scripts will be the IDs or IPs.

    Useful for piping into nn commands that take resource IDs as input, e.g. 

    `nn list host --tag web -o ids | nn delete host`
    ", default_value_t = false, global=true)]
    ids: bool,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    #[clap(name = "import", about = "Import an Nmap XML scan report into the database with optional site association and default acceptance of proposed imports")]
    ImportScan {
        #[clap(help = "Path to the Nmap XML file to import")]
        file: String,
        #[clap(short,long,help = "Accept default actions for all proposed imports without prompting", default_value_t = false)]
        accept_defaults: bool,
        #[clap(long, help = "Site name to associate with imported data (optional)", default_value = "red")]
        site : String,
        
    },
    #[clap(name = "list", about = "List resources with optional filtering and output formats", alias = "ls", alias = "l")]
    List {
        #[clap(short, long, help = "Filter hosts by site name", default_value = "red")]
        site: Option<String>,
        #[clap(short, long, help = "Filter by tag name (specifying multiple tags only shows resources with all specified tags)")]
        tag: Vec<String>,
        #[clap(flatten )]
        listing_types: ListingTypes,
        #[clap(subcommand)]
        filter: Option<ResourceTypesFilters>,
    },
    #[clap(name = "delete", about = "Delete resources with specified IDs.", alias = "del", alias = "rm", alias = "remove", alias = "d")]
    Delete {
        #[clap(help = "The type of resource to delete (host, address, service, network, note, tag, site)")]
        resource_type: String,
        #[clap(help = "The ID(s) of the resource(s) to delete, e.g. 1 2 3. Obtain with `nn list <resource> -i`. Otherwise reads IDs from stdin, one per line.")]
        ids: Vec<i32>,
    },
    #[clap(name = "note", about = "Edit the note for a resource in $EDITOR")]
    Note {
        #[clap(help = "Resource type: host, address, service, network")]
        resource_type: String,
        #[clap(help = "Resource ID")]
        id: i32,
    },
    #[clap(name = "tag", about = "Create or assign tags to resources")]
    Tag {
        #[clap(subcommand)]
        action: TagAction,
    },
    #[clap(name = "cred", about = "Add, update, or manage credentials")]
    Cred {
        #[clap(subcommand)]
        action: CredAction,
    },
}
// defaults to sqlite if not provided
pub fn establish_connection(args : &Args) -> Result<AnyConnection, NNError> {
    let database_url = args.database_url.as_deref().unwrap_or("sqlite://database.db");
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        let conn = diesel::PgConnection::establish(database_url)?;
        Ok(AnyConnection::Postgresql(conn))
    } else if database_url.starts_with("sqlite://") {
        use diesel::connection::SimpleConnection;
        let mut conn = diesel::SqliteConnection::establish(database_url)?;
        conn.batch_execute("PRAGMA foreign_keys = ON;")?;
        Ok(AnyConnection::Sqlite(conn))
    } else {
        Err(NNError::DatabaseError(diesel::result::Error::NotFound)) // or some custom error
    }
}
fn main() -> Result<(), NNError> {
    let args = Args::parse();
    match &args.command {
        Commands::ImportScan { .. } => import_cmd::import_cmd(&args)?,
        Commands::List { .. } => list_cmd::list_cmd(&args)?,
        Commands::Delete { .. } => delete_cmd::delete_command(&args, &mut establish_connection(&args)?)?,
        Commands::Note { .. } => note_cmd::note_command(&args, &mut establish_connection(&args)?)?,
        Commands::Tag { .. } => tag_cmd::tag_command(&args, &mut establish_connection(&args)?)?,
        Commands::Cred { .. } => cred_cmd::cred_command(&args, &mut establish_connection(&args)?)?,
    }
    Ok(())
}
