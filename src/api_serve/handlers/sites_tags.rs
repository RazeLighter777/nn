use actix_web::{delete, get, patch, post, web, HttpResponse};
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    api_serve::{db::DbPool, ListResponse},
    models::*,
    schema::{site, tag, tag_assignment, note},
    NNError,
};

#[derive(Deserialize)]
pub struct PaginationQ {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub q: Option<String>,
}
pub fn default_limit() -> i64 { 100 }

#[derive(Deserialize)]
pub struct NoteBody {
    pub text: String,
}

// ── Sites ─────────────────────────────────────────────────────────────────────

#[get("/sites")]
pub async fn list_sites(pool: web::Data<DbPool>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<ListResponse<Site>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = site::table.into_boxed();
        if let Some(ref s) = q.q { query = query.filter(site::name.like(format!("%{}%", s))); }
        query.limit(q.limit).offset(q.offset).load::<Site>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

#[post("/sites")]
pub async fn create_site(pool: web::Data<DbPool>, body: web::Json<NewSite>) -> actix_web::Result<web::Json<Site>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(site::table).values(&body.into_inner()).execute(&mut *conn)?;
        site::table.order(site::id.desc()).first::<Site>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/sites/{id}")]
pub async fn get_site(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Site>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        site::table.find(id).first::<Site>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[patch("/sites/{id}")]
pub async fn update_site(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<SiteChangeset>) -> actix_web::Result<web::Json<Site>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(site::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        site::table.find(id).first::<Site>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/sites/{id}")]
pub async fn delete_site(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(site::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

// ── Tags ──────────────────────────────────────────────────────────────────────

#[get("/tags")]
pub async fn list_tags(pool: web::Data<DbPool>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<ListResponse<Tag>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        let mut query = tag::table.into_boxed();
        if let Some(ref s) = q.q { query = query.filter(tag::name.like(format!("%{}%", s))); }
        query.limit(q.limit).offset(q.offset).load::<Tag>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

#[post("/tags")]
pub async fn create_tag(pool: web::Data<DbPool>, body: web::Json<NewTag>) -> actix_web::Result<web::Json<Tag>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(tag::table).values(&body.into_inner()).execute(&mut *conn)?;
        tag::table.order(tag::id.desc()).first::<Tag>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/tags/{id}")]
pub async fn get_tag(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Tag>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        tag::table.find(id).first::<Tag>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[patch("/tags/{id}")]
pub async fn update_tag(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<TagChangeset>) -> actix_web::Result<web::Json<Tag>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(tag::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        tag::table.find(id).first::<Tag>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/tags/{id}")]
pub async fn delete_tag(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(tag::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

// ── Tag assignments (flat) ────────────────────────────────────────────────────

#[get("/tag-assignments")]
pub async fn list_tag_assignments(pool: web::Data<DbPool>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<ListResponse<TagAssignment>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        tag_assignment::table.limit(q.limit).offset(q.offset)
            .load::<TagAssignment>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

// ── Generic tag sub-resource helpers ─────────────────────────────────────────

pub fn list_tags_for(conn: &mut crate::AnyConnection, col: &str, entity_id: i32) -> Result<Vec<TagAssignment>, NNError> {
    use crate::schema::tag_assignment::dsl as ta;
    match col {
        "service_id"    => ta::tag_assignment.filter(ta::service_id.eq(entity_id)).load(conn).map_err(Into::into),
        "host_id"       => ta::tag_assignment.filter(ta::host_id.eq(entity_id)).load(conn).map_err(Into::into),
        "address_id"    => ta::tag_assignment.filter(ta::address_id.eq(entity_id)).load(conn).map_err(Into::into),
        "network_id"    => ta::tag_assignment.filter(ta::network_id.eq(entity_id)).load(conn).map_err(Into::into),
        "credential_id" => ta::tag_assignment.filter(ta::credential_id.eq(entity_id)).load(conn).map_err(Into::into),
        _ => Err(NNError::DatabaseError(diesel::result::Error::NotFound)),
    }
}

pub fn assign_tag(conn: &mut crate::AnyConnection, tag_id: i32, col: &str, entity_id: i32) -> Result<TagAssignment, NNError> {
    use crate::schema::tag_assignment::dsl as ta;
    let new = match col {
        "service_id"    => NewTagAssignment { tag_id, service_id: Some(entity_id), address_id: None, host_id: None, network_id: None, credential_id: None },
        "host_id"       => NewTagAssignment { tag_id, service_id: None, address_id: None, host_id: Some(entity_id), network_id: None, credential_id: None },
        "address_id"    => NewTagAssignment { tag_id, service_id: None, address_id: Some(entity_id), host_id: None, network_id: None, credential_id: None },
        "network_id"    => NewTagAssignment { tag_id, service_id: None, address_id: None, host_id: None, network_id: Some(entity_id), credential_id: None },
        "credential_id" => NewTagAssignment { tag_id, service_id: None, address_id: None, host_id: None, network_id: None, credential_id: Some(entity_id) },
        _ => return Err(NNError::DatabaseError(diesel::result::Error::NotFound)),
    };
    diesel::insert_into(ta::tag_assignment).values(&new).execute(conn)?;
    ta::tag_assignment.order(ta::id.desc()).first(conn).map_err(Into::into)
}

pub fn remove_tag_from(conn: &mut crate::AnyConnection, tag_id: i32, col: &str, entity_id: i32) -> Result<(), NNError> {
    use crate::schema::tag_assignment::dsl as ta;
    let n = match col {
        "service_id"    => diesel::delete(ta::tag_assignment.filter(ta::tag_id.eq(tag_id)).filter(ta::service_id.eq(entity_id))).execute(conn)?,
        "host_id"       => diesel::delete(ta::tag_assignment.filter(ta::tag_id.eq(tag_id)).filter(ta::host_id.eq(entity_id))).execute(conn)?,
        "address_id"    => diesel::delete(ta::tag_assignment.filter(ta::tag_id.eq(tag_id)).filter(ta::address_id.eq(entity_id))).execute(conn)?,
        "network_id"    => diesel::delete(ta::tag_assignment.filter(ta::tag_id.eq(tag_id)).filter(ta::network_id.eq(entity_id))).execute(conn)?,
        "credential_id" => diesel::delete(ta::tag_assignment.filter(ta::tag_id.eq(tag_id)).filter(ta::credential_id.eq(entity_id))).execute(conn)?,
        _ => return Err(NNError::DatabaseError(diesel::result::Error::NotFound)),
    };
    if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
    Ok(())
}

// ── Notes (flat) ──────────────────────────────────────────────────────────────

#[get("/notes")]
pub async fn list_notes(pool: web::Data<DbPool>, q: web::Query<PaginationQ>) -> actix_web::Result<web::Json<ListResponse<Note>>> {
    let rows = web::block(move || {
        let mut conn = pool.lock().unwrap();
        note::table.limit(q.limit).offset(q.offset).load::<Note>(&mut *conn).map_err(NNError::from)
    }).await??;
    let total = rows.len();
    Ok(web::Json(ListResponse { total, items: rows }))
}

#[post("/notes")]
pub async fn create_note(pool: web::Data<DbPool>, body: web::Json<NewNote>) -> actix_web::Result<web::Json<Note>> {
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::insert_into(note::table).values(&body.into_inner()).execute(&mut *conn)?;
        note::table.order(note::id.desc()).first::<Note>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[get("/notes/{id}")]
pub async fn get_note(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        note::table.find(id).first::<Note>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[patch("/notes/{id}")]
pub async fn update_note(pool: web::Data<DbPool>, path: web::Path<i32>, body: web::Json<NoteChangeset>) -> actix_web::Result<web::Json<Note>> {
    let id = path.into_inner();
    let row = web::block(move || {
        let mut conn = pool.lock().unwrap();
        diesel::update(note::table.find(id)).set(&body.into_inner()).execute(&mut *conn)?;
        note::table.find(id).first::<Note>(&mut *conn).map_err(NNError::from)
    }).await??;
    Ok(web::Json(row))
}

#[delete("/notes/{id}")]
pub async fn delete_note(pool: web::Data<DbPool>, path: web::Path<i32>) -> actix_web::Result<HttpResponse> {
    let id = path.into_inner();
    web::block(move || {
        let mut conn = pool.lock().unwrap();
        let n = diesel::delete(note::table.find(id)).execute(&mut *conn)?;
        if n == 0 { return Err(NNError::DatabaseError(diesel::result::Error::NotFound)); }
        Ok::<_, NNError>(())
    }).await??;
    Ok(HttpResponse::NoContent().finish())
}

pub fn list_notes_for(conn: &mut crate::AnyConnection, col: &str, entity_id: i32, limit: i64, offset: i64) -> Result<Vec<Note>, NNError> {
    use crate::schema::note::dsl as n;
    match col {
        "service_id"    => n::note.filter(n::service_id.eq(entity_id)).limit(limit).offset(offset).load(conn).map_err(Into::into),
        "host_id"       => n::note.filter(n::host_id.eq(entity_id)).limit(limit).offset(offset).load(conn).map_err(Into::into),
        "address_id"    => n::note.filter(n::address_id.eq(entity_id)).limit(limit).offset(offset).load(conn).map_err(Into::into),
        "network_id"    => n::note.filter(n::network_id.eq(entity_id)).limit(limit).offset(offset).load(conn).map_err(Into::into),
        "credential_id" => n::note.filter(n::credential_id.eq(entity_id)).limit(limit).offset(offset).load(conn).map_err(Into::into),
        _ => Err(NNError::DatabaseError(diesel::result::Error::NotFound)),
    }
}

pub fn create_note_for(conn: &mut crate::AnyConnection, text: String, col: &str, entity_id: i32) -> Result<Note, NNError> {
    use crate::schema::note::dsl as n;
    let new = match col {
        "service_id"    => NewNote { text, service_id: Some(entity_id), address_id: None, host_id: None, network_id: None, credential_id: None },
        "host_id"       => NewNote { text, service_id: None, address_id: None, host_id: Some(entity_id), network_id: None, credential_id: None },
        "address_id"    => NewNote { text, service_id: None, address_id: Some(entity_id), host_id: None, network_id: None, credential_id: None },
        "network_id"    => NewNote { text, service_id: None, address_id: None, host_id: None, network_id: Some(entity_id), credential_id: None },
        "credential_id" => NewNote { text, service_id: None, address_id: None, host_id: None, network_id: None, credential_id: Some(entity_id) },
        _ => return Err(NNError::DatabaseError(diesel::result::Error::NotFound)),
    };
    diesel::insert_into(n::note).values(&new).execute(conn)?;
    n::note.order(n::id.desc()).first(conn).map_err(Into::into)
}
