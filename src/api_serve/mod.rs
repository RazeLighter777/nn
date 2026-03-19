pub mod db;
pub mod handlers;

use actix_web::{web, App, HttpServer};
use actix_web::http::StatusCode;

use crate::NNError;
use db::DbPool;
use handlers::{hosts_addresses::*, services_creds::*, sites_tags::*};

// ── Error conversion ──────────────────────────────────────────────────────────

impl actix_web::ResponseError for NNError {
    fn status_code(&self) -> StatusCode {
        match self {
            NNError::DatabaseError(diesel::result::Error::NotFound) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// ── App factory ───────────────────────────────────────────────────────────────

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // sites
        .service(list_sites).service(create_site).service(get_site).service(update_site).service(delete_site)
        // tags
        .service(list_tags).service(create_tag).service(get_tag).service(update_tag).service(delete_tag)
        .service(list_tag_assignments)
        // networks
        .service(list_networks).service(create_network).service(get_network).service(update_network).service(delete_network)
        .service(list_network_tags).service(add_network_tag).service(remove_network_tag)
        .service(list_network_notes).service(add_network_note)
        // hosts
        .service(list_hosts).service(create_host).service(get_host).service(update_host).service(delete_host)
        .service(list_host_addresses).service(list_host_services)
        .service(list_host_tags).service(add_host_tag).service(remove_host_tag)
        .service(list_host_notes).service(add_host_note)
        // addresses
        .service(list_addresses).service(create_address).service(get_address).service(update_address).service(delete_address)
        .service(list_address_tags).service(add_address_tag).service(remove_address_tag)
        .service(list_address_notes).service(add_address_note)
        // services
        .service(list_services).service(create_service).service(get_service).service(update_service).service(delete_service)
        .service(list_service_tags).service(add_service_tag).service(remove_service_tag)
        .service(list_service_notes).service(add_service_note)
        .service(list_service_credentials).service(link_service_credential).service(unlink_service_credential)
        // credentials
        .service(list_credentials).service(create_credential).service(get_credential).service(update_credential).service(delete_credential)
        .service(list_cred_tags).service(add_cred_tag).service(remove_cred_tag)
        .service(list_cred_notes).service(add_cred_note)
        // notes
        .service(list_notes).service(create_note).service(get_note).service(update_note).service(delete_note);
}

pub async fn run_server(pool: DbPool, bind: &str) -> std::io::Result<()> {
    let data = web::Data::new(pool);
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                let msg = err.to_string();
                actix_web::error::InternalError::from_response(
                    err,
                    actix_web::HttpResponse::BadRequest().body(msg),
                ).into()
            }))
            .service(web::scope("/api/v1").configure(configure_routes))
    })
    .bind(bind)?
    .run()
    .await
}

pub fn api_serve_command(args: &crate::Args) -> Result<(), NNError> {
    let crate::Commands::ApiServe { bind, database_url } = &args.command else { unreachable!() };
    // Per-subcommand flag overrides the global flag, which already falls back to DATABASE_URL env.
    // resolve_database_url then handles PGHOST/... env vars and the sqlite default.
    let explicit = database_url.as_deref().or(args.database_url.as_deref());
    let url = crate::resolve_database_url(explicit);
    let pool = db::new_pool(&url)?;
    actix_web::rt::System::new().block_on(run_server(pool, bind))
        .map_err(NNError::IoError)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use diesel::prelude::*;
    use std::sync::{Arc, Mutex};
    use crate::AnyConnection;

    const SCHEMA_SQL: &str = include_str!("../../migrations/sqlite/2026-03-19-200000-0000_initial/up.sql");

    fn test_pool() -> DbPool {
        use diesel::connection::SimpleConnection;
        let mut conn = diesel::SqliteConnection::establish(":memory:").expect("in-memory sqlite");
        conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
        conn.batch_execute(SCHEMA_SQL).unwrap();
        Arc::new(Mutex::new(AnyConnection::Sqlite(conn)))
    }

    fn app_with_pool(pool: DbPool) -> actix_web::App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        App::new()
            .app_data(web::Data::new(pool))
            .service(web::scope("/api/v1").configure(configure_routes))
    }

    // ── Site CRUD ─────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_list_sites_empty() {
        let svc = test::init_service(app_with_pool(test_pool())).await;
        let req = test::TestRequest::get().uri("/api/v1/sites").to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());
        let body: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert!(body.is_empty());
    }

    #[actix_web::test]
    async fn test_create_and_get_site() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/sites")
            .set_json(serde_json::json!({"name": "alpha"}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());
        let created: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(created["name"], "alpha");
        let id = created["id"].as_i64().unwrap();

        let req = test::TestRequest::get().uri(&format!("/api/v1/sites/{id}")).to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());
        let got: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(got["name"], "alpha");
    }

    #[actix_web::test]
    async fn test_update_site() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/sites")
            .set_json(serde_json::json!({"name": "beta"}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        let created: serde_json::Value = test::read_body_json(resp).await;
        let id = created["id"].as_i64().unwrap();

        let req = test::TestRequest::patch()
            .uri(&format!("/api/v1/sites/{id}"))
            .set_json(serde_json::json!({"name": "gamma"}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());
        let updated: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(updated["name"], "gamma");
    }

    #[actix_web::test]
    async fn test_delete_site() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/sites")
            .set_json(serde_json::json!({"name": "to-delete"}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        let created: serde_json::Value = test::read_body_json(resp).await;
        let id = created["id"].as_i64().unwrap();

        let req = test::TestRequest::delete().uri(&format!("/api/v1/sites/{id}")).to_request();
        let resp = test::call_service(&svc, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);

        let req = test::TestRequest::get().uri(&format!("/api/v1/sites/{id}")).to_request();
        let resp = test::call_service(&svc, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_site_not_found() {
        let svc = test::init_service(app_with_pool(test_pool())).await;
        let req = test::TestRequest::get().uri("/api/v1/sites/9999").to_request();
        let resp = test::call_service(&svc, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    // ── Tag CRUD ──────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_tag_crud() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/tags")
            .set_json(serde_json::json!({"name": "web"}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());
        let tag: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(tag["name"], "web");
        let tag_id = tag["id"].as_i64().unwrap();

        let req = test::TestRequest::patch()
            .uri(&format!("/api/v1/tags/{tag_id}"))
            .set_json(serde_json::json!({"name": "web-updated"}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());

        let req = test::TestRequest::delete().uri(&format!("/api/v1/tags/{tag_id}")).to_request();
        let resp = test::call_service(&svc, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    }

    // ── Network + tags sub-resource ───────────────────────────────────────────

    #[actix_web::test]
    async fn test_network_with_tag_assignment() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        // Create site
        let req = test::TestRequest::post().uri("/api/v1/sites").set_json(serde_json::json!({"name": "s1"})).to_request();
        let site: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let site_id = site["id"].as_i64().unwrap();

        // Create network
        let req = test::TestRequest::post().uri("/api/v1/networks").set_json(serde_json::json!({"site_id": site_id, "name": "office"})).to_request();
        let net: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let net_id = net["id"].as_i64().unwrap();

        // Create tag
        let req = test::TestRequest::post().uri("/api/v1/tags").set_json(serde_json::json!({"name": "internal"})).to_request();
        let tag: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let tag_id = tag["id"].as_i64().unwrap();

        // Assign tag to network
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/networks/{net_id}/tags"))
            .set_json(serde_json::json!({"tag_id": tag_id}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());

        // List tags on network
        let req = test::TestRequest::get().uri(&format!("/api/v1/networks/{net_id}/tags")).to_request();
        let resp = test::call_service(&svc, req).await;
        let tags: Vec<serde_json::Value> = test::read_body_json(resp).await;
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0]["tag_id"], tag_id);

        // Remove tag
        let req = test::TestRequest::delete().uri(&format!("/api/v1/networks/{net_id}/tags/{tag_id}")).to_request();
        let resp = test::call_service(&svc, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    }

    // ── Host + address + service chain ────────────────────────────────────────

    #[actix_web::test]
    async fn test_host_address_service_chain() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post().uri("/api/v1/sites").set_json(serde_json::json!({"name": "red"})).to_request();
        let site: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let site_id = site["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/networks").set_json(serde_json::json!({"site_id": site_id, "name": "lan"})).to_request();
        let net: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let net_id = net["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/hosts").set_json(serde_json::json!({"site_id": site_id, "name": "web01"})).to_request();
        let host: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let host_id = host["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/addresses").set_json(serde_json::json!({
            "host_id": host_id, "network_id": net_id, "ip": "10.0.0.1", "ip_family": 4, "netmask": 24
        })).to_request();
        let addr: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let addr_id = addr["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/services").set_json(serde_json::json!({
            "site_id": site_id, "address_id": addr_id, "port": 80,
            "ip_proto_number": 6, "state": "open", "name": "http"
        })).to_request();
        let svc_row: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        assert_eq!(svc_row["port"], 80);
        let svc_id = svc_row["id"].as_i64().unwrap();

        // List services for host via /hosts/{id}/services
        let req = test::TestRequest::get().uri(&format!("/api/v1/hosts/{host_id}/services")).to_request();
        let svcs: Vec<serde_json::Value> = test::read_body_json(test::call_service(&svc, req).await).await;
        assert_eq!(svcs.len(), 1);
        assert_eq!(svcs[0]["id"], svc_id);
    }

    // ── Credential linked to service ──────────────────────────────────────────

    #[actix_web::test]
    async fn test_credential_service_link() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post().uri("/api/v1/sites").set_json(serde_json::json!({"name": "s"})).to_request();
        let site: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let site_id = site["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/networks").set_json(serde_json::json!({"site_id": site_id, "name": "n"})).to_request();
        let net: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let net_id = net["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/hosts").set_json(serde_json::json!({"site_id": site_id, "name": "h"})).to_request();
        let host: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let host_id = host["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/addresses").set_json(serde_json::json!({
            "host_id": host_id, "network_id": net_id, "ip": "10.0.0.2", "ip_family": 4, "netmask": 24
        })).to_request();
        let addr: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let addr_id = addr["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/services").set_json(serde_json::json!({
            "site_id": site_id, "address_id": addr_id, "port": 22, "ip_proto_number": 6, "state": "open", "name": "ssh"
        })).to_request();
        let svc_row: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let svc_id = svc_row["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/credentials").set_json(serde_json::json!({
            "username": "root", "password": "toor", "hash": null
        })).to_request();
        let cred: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let cred_id = cred["id"].as_i64().unwrap();
        assert_eq!(cred["username"], "root");

        // Link cred to service
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/services/{svc_id}/credentials"))
            .set_json(serde_json::json!({"credential_id": cred_id}))
            .to_request();
        let resp = test::call_service(&svc, req).await;
        assert!(resp.status().is_success());

        // List creds for service
        let req = test::TestRequest::get().uri(&format!("/api/v1/services/{svc_id}/credentials")).to_request();
        let creds: Vec<serde_json::Value> = test::read_body_json(test::call_service(&svc, req).await).await;
        assert_eq!(creds.len(), 1);

        // Unlink
        let req = test::TestRequest::delete().uri(&format!("/api/v1/services/{svc_id}/credentials/{cred_id}")).to_request();
        let resp = test::call_service(&svc, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::NO_CONTENT);
    }

    // ── Notes ─────────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn test_note_on_host() {
        let svc = test::init_service(app_with_pool(test_pool())).await;

        let req = test::TestRequest::post().uri("/api/v1/sites").set_json(serde_json::json!({"name": "sr"})).to_request();
        let site: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let site_id = site["id"].as_i64().unwrap();

        let req = test::TestRequest::post().uri("/api/v1/hosts").set_json(serde_json::json!({"site_id": site_id, "name": "noted"})).to_request();
        let host: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        let host_id = host["id"].as_i64().unwrap();

        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/hosts/{host_id}/notes"))
            .set_json(serde_json::json!({"text": "important finding"}))
            .to_request();
        let note: serde_json::Value = test::read_body_json(test::call_service(&svc, req).await).await;
        assert_eq!(note["text"], "important finding");
        assert_eq!(note["host_id"], host_id);

        let req = test::TestRequest::get().uri(&format!("/api/v1/hosts/{host_id}/notes")).to_request();
        let notes: Vec<serde_json::Value> = test::read_body_json(test::call_service(&svc, req).await).await;
        assert_eq!(notes.len(), 1);
    }
}
