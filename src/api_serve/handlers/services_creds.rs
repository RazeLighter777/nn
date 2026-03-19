use actix_web::{delete, get, patch, post, web, HttpResponse};
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    api_serve::{
        db::DbPool,
        handlers::sites_tags::{PaginationQ, NoteBody, list_tags_for, assign_tag, remove_tag_from, list_notes_for, create_note_for},
        ListResponse,
    },
    models::*,
    schema::{service, address, credential, credential_service},
    NNError,
};

// ── Services ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ServiceQuery {
    #[serde(default = "crate::api_serve::handlers::sites_tags::default_limit")] pub limit: i64,
    #[serde(default)] pub offset: i64,
    pub q: Option<String>, pub site_id: Option<i32>, pub address_id: Option<i32>,
    pub port: Option<i32>, pub state: Option<String>, pub ip_proto_number: Option<i32>,
}

#[get("/services")]
pub async fn list_services(pool: web::Data<DbPool>, q: web::Query<ServiceQuery>) -> actix_web::Result<web::Json<ListResponse<Service>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = service::table.into_boxed();
        if let Some(ref s) = q.q { query = query.filter(service::name.like(format!("%{}%", s)).or(service::product.like(format!("%{}%", s)))); }
        if let Some(v) = q.site_id { query = query.filter(service::site_id.eq(v)); }
        if let Some(v) = q.address_id { query = query.filter(service::address_id.eq(v)); }
        if let Some(v) = q.port { query = query.filter(service::port.eq(v)); }
        if let Some(ref v) = q.state { query = query.filter(service::state.eq(v)); }
        if let Some(v) = q.ip_proto_number { query = query.filter(service::ip_proto_number.eq(v)); }
        query.limit(q.limit).offset(q.offset).load::<Service>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

#[post("/services")]
pub async fn create_service(pool: web::Data<DbPool>, body: web::Json<NewService>) -> actix_web::Result<web::Json<Service>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(service::table).values(&body.into_inner()).execute(&mut *conn)?;
        service::table.order(service::id.desc()).first::<Service>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/services/{id}")]
pub async fn get_service(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Service>> {
    let id = path.into_inner();
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); service::table.find(id).first::<Service>(&mut *conn).map_err(NNError::from) }).await??;
    Ok(web::Json(row))
}

#[patch("/services/{id}")]
pub async fn update_service(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<ServiceChangeset>) -> actix_web::Result<web::Json<Service>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(service::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        service::table.find(id).first::<Service>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/services/{id}")]
pub async fn delete_service(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(service::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/hosts/{id}/services")]
pub async fn list_host_services(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<ListResponse<Service>>> {
    let id = path.into_inner();
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        service::table.inner_join(address::table)
            .filter(address::host_id.eq(id))
            .limit(q.limit).offset(q.offset)
            .select(Service::as_select())
            .load::<Service>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

// ── Service tags ──────────────────────────────────────────────────────────────

#[get("/services/{id}/tags")]
pub async fn list_service_tags(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Vec<TagAssignment>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_tags_for(&mut *conn, "service_id", id) }).await??;
    Ok(web::Json(rows))
}

#[post("/services/{id}/tags")]
pub async fn add_service_tag(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<serde_json::Value>) -> actix_web::Result<web::Json<TagAssignment>> {
    let id = path.into_inner();
    let tag_id = body["tag_id"].as_i64().ok_or_else(|| actix_web::error::ErrorBadRequest("tag_id required"))? as i32;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); assign_tag(&mut *conn, tag_id, "service_id", id) }).await??;
    Ok(web::Json(row))
}

#[delete("/services/{id}/tags/{tag_id}")]
pub async fn remove_service_tag(pool: web::Data<DbPool>, path: web::Path<(i32,i32)>) -> actix_web::Result<HttpResponse> {
    let (id, tag_id) = path.into_inner();
    web::block(move || { let mut conn = pool.lock().unwrap(); remove_tag_from(&mut *conn, tag_id, "service_id", id) }).await??;
    Ok(HttpResponse::NoContent().finish())
}

// ── Service notes ─────────────────────────────────────────────────────────────

#[get("/services/{id}/notes")]
pub async fn list_service_notes(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Note>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_notes_for(&mut *conn, "service_id", id, q.limit, q.offset) }).await??;
    Ok(web::Json(rows))
}

#[post("/services/{id}/notes")]
pub async fn add_service_note(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NoteBody>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let text = body.into_inner().text;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); create_note_for(&mut *conn, text, "service_id", id) }).await??;
    Ok(web::Json(row))
}

// ── Service credentials ───────────────────────────────────────────────────────

#[get("/services/{id}/credentials")]
pub async fn list_service_credentials(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Credential>>> {
    let id = path.into_inner();
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        credential::table.inner_join(credential_service::table)
            .filter(credential_service::service_id.eq(id))
            .limit(q.limit).offset(q.offset)
            .select(Credential::as_select())
            .load::<Credential>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(rows))
}

#[post("/services/{id}/credentials")]
pub async fn link_service_credential(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<serde_json::Value>) -> actix_web::Result<web::Json<CredentialService>> {
    let svc_id = path.into_inner();
    let cred_id = body["credential_id"].as_i64().ok_or_else(|| actix_web::error::ErrorBadRequest("credential_id required"))? as i32;
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let new = NewCredentialService { credential_id: cred_id, service_id: svc_id };
        diesel::insert_into(credential_service::table).values(&new).execute(&mut *conn)?;
        credential_service::table.order(credential_service::id.desc()).first::<CredentialService>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/services/{svc_id}/credentials/{cred_id}")]
pub async fn unlink_service_credential(pool: web::Data<DbPool>, path: web::Path<(i32,i32)>) -> actix_web::Result<HttpResponse> {
    let (svc_id, cred_id) = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(
            credential_service::table
                .filter(credential_service::service_id.eq(svc_id))
                .filter(credential_service::credential_id.eq(cred_id))
        ).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

// ── Credentials ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CredentialQuery {
    #[serde(default = "crate::api_serve::handlers::sites_tags::default_limit")] pub limit: i64,
    #[serde(default)] pub offset: i64,
    pub username: Option<String>,
}

#[get("/credentials")]
pub async fn list_credentials(pool: web::Data<DbPool>, q: web::Query<CredentialQuery>) -> actix_web::Result<web::Json<ListResponse<Credential>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = credential::table.into_boxed();
        if let Some(ref u) = q.username { query = query.filter(credential::username.eq(u)); }
        query.limit(q.limit).offset(q.offset).load::<Credential>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

#[post("/credentials")]
pub async fn create_credential(pool: web::Data<DbPool>, body: web::Json<NewCredential>) -> actix_web::Result<web::Json<Credential>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(credential::table).values(&body.into_inner()).execute(&mut *conn)?;
        credential::table.order(credential::id.desc()).first::<Credential>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/credentials/{id}")]
pub async fn get_credential(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Credential>> {
    let id = path.into_inner();
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); credential::table.find(id).first::<Credential>(&mut *conn).map_err(NNError::from) }).await??;
    Ok(web::Json(row))
}

#[patch("/credentials/{id}")]
pub async fn update_credential(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<CredentialChangeset>) -> actix_web::Result<web::Json<Credential>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(credential::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        credential::table.find(id).first::<Credential>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/credentials/{id}")]
pub async fn delete_credential(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(credential::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/credentials/{id}/tags")]
pub async fn list_cred_tags(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Vec<TagAssignment>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_tags_for(&mut *conn, "credential_id", id) }).await??;
    Ok(web::Json(rows))
}

#[post("/credentials/{id}/tags")]
pub async fn add_cred_tag(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<serde_json::Value>) -> actix_web::Result<web::Json<TagAssignment>> {
    let id = path.into_inner();
    let tag_id = body["tag_id"].as_i64().ok_or_else(|| actix_web::error::ErrorBadRequest("tag_id required"))? as i32;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); assign_tag(&mut *conn, tag_id, "credential_id", id) }).await??;
    Ok(web::Json(row))
}

#[delete("/credentials/{id}/tags/{tag_id}")]
pub async fn remove_cred_tag(pool: web::Data<DbPool>, path: web::Path<(i32,i32)>) -> actix_web::Result<HttpResponse> {
    let (id, tag_id) = path.into_inner();
    web::block(move || { let mut conn = pool.lock().unwrap(); remove_tag_from(&mut *conn, tag_id, "credential_id", id) }).await??;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/credentials/{id}/notes")]
pub async fn list_cred_notes(pool: web::Data<DbPool>, path: web::Path<i32>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<Vec<Note>>> {
    let id = path.into_inner();
    let rows = web::block(move || { let mut conn = pool.lock().unwrap(); list_notes_for(&mut *conn, "credential_id", id, q.limit, q.offset) }).await??;
    Ok(web::Json(rows))
}

#[post("/credentials/{id}/notes")]
pub async fn add_cred_note(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NoteBody>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let text = body.into_inner().text;
    let row = web::block(move || { let mut conn = pool.lock().unwrap(); create_note_for(&mut *conn, text, "credential_id", id) }).await??;
    Ok(web::Json(row))
}
