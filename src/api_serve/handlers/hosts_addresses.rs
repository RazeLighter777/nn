use actix_web::{delete, get, patch, post, web, HttpResponse};
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    api_serve::{
        db::DbPool,
        handlers::sites_tags::{PaginationQ, NoteBody, list_tags_for, assign_tag, remove_tag_from, list_notes_for, create_note_for},
    },
    models::*,
    schema::{host, address, network},
    NNError,
};

// ── Networks ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NetworkQuery { #[serde(default = "crate::api_serve::handlers::sites_tags::default_limit")] pub limit: i64, #[serde(default)] pub offset: i64, pub q: Option<String>, pub site_id: Option<i32> }

#[get("/networks")]
pub async fn list_networks(pool: web::Data<DbPool>, q: web::Query<NetworkQuery>) -> actix_web::Result<web::Json<Vec<Network>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = network::table.into_boxed();
        if let Some(ref s) = q.q { query = query.filter(network::name.like(format!("%{}%", s))); }
        if let Some(sid) = q.site_id { query = query.filter(network::site_id.eq(sid)); }
        query.limit(q.limit).offset(q.offset).load::<Network>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(rows))
}

#[post("/networks")]
pub async fn create_network(pool: web::Data<DbPool>, body: web::Json<NewNetwork>) -> actix_web::Result<web::Json<Network>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(network::table).values(&body.into_inner()).execute(&mut *conn)?;
        network::table.order(network::id.desc()).first::<Network>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/networks/{id}")]
pub async fn get_network(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Network>> {
    let id = path.into_inner();
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); network::table.find(id).first::<Network>(&mut *conn).map_err(NNError::from) }).await??;
    Ok(web::Json(row))
}

#[patch("/networks/{id}")]
pub async fn update_network(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NetworkChangeset>) -> actix_web::Result<web::Json<Network>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(network::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        network::table.find(id).first::<Network>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/networks/{id}")]
pub async fn delete_network(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(network::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/networks/{id}/tags")]
pub async fn list_network_tags(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Vec<TagAssignment>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_tags_for(&mut *conn, "network_id", id) }).await??;
    Ok(web::Json(rows))
}

#[post("/networks/{id}/tags")]
pub async fn add_network_tag(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<serde_json::Value>) -> actix_web::Result<web::Json<TagAssignment>> {
    let id = path.into_inner();
    let tag_id = body["tag_id"].as_i64().ok_or_else(|| actix_web::error::ErrorBadRequest("tag_id required"))? as i32;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); assign_tag(&mut *conn, tag_id, "network_id", id) }).await??;
    Ok(web::Json(row))
}

#[delete("/networks/{id}/tags/{tag_id}")]
pub async fn remove_network_tag(pool: web::Data<DbPool>, path: web::Path<(i32,i32)>) -> actix_web::Result<HttpResponse> {
    let (id, tag_id) = path.into_inner();
    web::block(move || { let mut conn = pool.lock().unwrap(); remove_tag_from(&mut *conn, tag_id, "network_id", id) }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/networks/{id}/notes")]
pub async fn list_network_notes(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Note>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_notes_for(&mut *conn, "network_id", id, q.limit, q.offset) }).await??;
    Ok(web::Json(rows))
}

#[post("/networks/{id}/notes")]
pub async fn add_network_note(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NoteBody>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let text = body.into_inner().text;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); create_note_for(&mut *conn, text, "network_id", id) }).await??;
    Ok(web::Json(row))
}

// ── Hosts ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct HostQuery { #[serde(default = "crate::api_serve::handlers::sites_tags::default_limit")] pub limit: i64, #[serde(default)] pub offset: i64, pub q: Option<String>, pub site_id: Option<i32>, pub os_type: Option<String> }

#[get("/hosts")]
pub async fn list_hosts(pool: web::Data<DbPool>, q: web::Query<HostQuery>) -> actix_web::Result<web::Json<Vec<Host>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = host::table.into_boxed();
        if let Some(ref s) = q.q { query = query.filter(host::name.like(format!("%{}%", s))); }
        if let Some(sid) = q.site_id { query = query.filter(host::site_id.eq(sid)); }
        if let Some(ref ot) = q.os_type { query = query.filter(host::os_type.eq(ot)); }
        query.limit(q.limit).offset(q.offset).load::<Host>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(rows))
}

#[post("/hosts")]
pub async fn create_host(pool: web::Data<DbPool>, body: web::Json<NewHost>) -> actix_web::Result<web::Json<Host>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(host::table).values(&body.into_inner()).execute(&mut *conn)?;
        host::table.order(host::id.desc()).first::<Host>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/hosts/{id}")]
pub async fn get_host(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Host>> {
    let id = path.into_inner();
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); host::table.find(id).first::<Host>(&mut *conn).map_err(NNError::from) }).await??;
    Ok(web::Json(row))
}

#[patch("/hosts/{id}")]
pub async fn update_host(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<HostChangeset>) -> actix_web::Result<web::Json<Host>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(host::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        host::table.find(id).first::<Host>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/hosts/{id}")]
pub async fn delete_host(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(host::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/hosts/{id}/tags")]
pub async fn list_host_tags(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Vec<TagAssignment>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_tags_for(&mut *conn, "host_id", id) }).await??;
    Ok(web::Json(rows))
}

#[post("/hosts/{id}/tags")]
pub async fn add_host_tag(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<serde_json::Value>) -> actix_web::Result<web::Json<TagAssignment>> {
    let id = path.into_inner();
    let tag_id = body["tag_id"].as_i64().ok_or_else(|| actix_web::error::ErrorBadRequest("tag_id required"))? as i32;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); assign_tag(&mut *conn, tag_id, "host_id", id) }).await??;
    Ok(web::Json(row))
}

#[delete("/hosts/{id}/tags/{tag_id}")]
pub async fn remove_host_tag(pool: web::Data<DbPool>, path: web::Path<(i32,i32)>) -> actix_web::Result<HttpResponse> {
    let (id, tag_id) = path.into_inner();
    web::block(move || { let mut conn = pool.lock().unwrap(); remove_tag_from(&mut *conn, tag_id, "host_id", id) }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/hosts/{id}/notes")]
pub async fn list_host_notes(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Note>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_notes_for(&mut *conn, "host_id", id, q.limit, q.offset) }).await??;
    Ok(web::Json(rows))
}

#[post("/hosts/{id}/notes")]
pub async fn add_host_note(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NoteBody>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let text = body.into_inner().text;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); create_note_for(&mut *conn, text, "host_id", id) }).await??;
    Ok(web::Json(row))
}

// ── Addresses ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddressQuery { #[serde(default = "crate::api_serve::handlers::sites_tags::default_limit")] pub limit: i64, #[serde(default)] pub offset: i64, pub host_id: Option<i32>, pub network_id: Option<i32>, pub ip: Option<String>, pub ip_family: Option<i32> }

#[get("/addresses")]
pub async fn list_addresses(pool: web::Data<DbPool>, q: web::Query<AddressQuery>) -> actix_web::Result<web::Json<Vec<Address>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = address::table.into_boxed();
        if let Some(hid) = q.host_id { query = query.filter(address::host_id.eq(hid)); }
        if let Some(nid) = q.network_id { query = query.filter(address::network_id.eq(nid)); }
        if let Some(ref ip) = q.ip { query = query.filter(address::ip.like(format!("{}%", ip))); }
        if let Some(fam) = q.ip_family { query = query.filter(address::ip_family.eq(fam)); }
        query.limit(q.limit).offset(q.offset).load::<Address>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(rows))
}

#[post("/addresses")]
pub async fn create_address(pool: web::Data<DbPool>, body: web::Json<NewAddress>) -> actix_web::Result<web::Json<Address>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(address::table).values(&body.into_inner()).execute(&mut *conn)?;
        address::table.order(address::id.desc()).first::<Address>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/addresses/{id}")]
pub async fn get_address(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Address>> {
    let id = path.into_inner();
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); address::table.find(id).first::<Address>(&mut *conn).map_err(NNError::from) }).await??;
    Ok(web::Json(row))
}

#[patch("/addresses/{id}")]
pub async fn update_address(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<AddressChangeset>) -> actix_web::Result<web::Json<Address>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(address::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        address::table.find(id).first::<Address>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/addresses/{id}")]
pub async fn delete_address(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(address::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/hosts/{id}/addresses")]
pub async fn list_host_addresses(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Address>>> {
    let id = path.into_inner();
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        address::table.filter(address::host_id.eq(id)).limit(q.limit).offset(q.offset).load::<Address>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(rows))
}

#[get("/addresses/{id}/tags")]
pub async fn list_address_tags(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Vec<TagAssignment>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_tags_for(&mut *conn, "address_id", id) }).await??;
    Ok(web::Json(rows))
}

#[post("/addresses/{id}/tags")]
pub async fn add_address_tag(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<serde_json::Value>) -> actix_web::Result<web::Json<TagAssignment>> {
    let id = path.into_inner();
    let tag_id = body["tag_id"].as_i64().ok_or_else(|| actix_web::error::ErrorBadRequest("tag_id required"))? as i32;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); assign_tag(&mut *conn, tag_id, "address_id", id) }).await??;
    Ok(web::Json(row))
}

#[delete("/addresses/{id}/tags/{tag_id}")]
pub async fn remove_address_tag(pool: web::Data<DbPool>, path: web::Path<(i32,i32)>) -> actix_web::Result<HttpResponse> {
    let (id, tag_id) = path.into_inner();
    web::block(move || { let mut conn = pool.lock().unwrap(); remove_tag_from(&mut *conn, tag_id, "address_id", id) }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/addresses/{id}/notes")]
pub async fn list_address_notes(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Note>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_notes_for(&mut *conn, "address_id", id, q.limit, q.offset) }).await??;
    Ok(web::Json(rows))
}

#[post("/addresses/{id}/notes")]
pub async fn add_address_note(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NoteBody>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let text = body.into_inner().text;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); create_note_for(&mut *conn, text, "address_id", id) }).await??;
    Ok(web::Json(row))
}
