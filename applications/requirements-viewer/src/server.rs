use crate::{Error, Store};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
struct Session {
    store: Arc<Mutex<Store>>,
    token: String,
    authority: String,
}

#[derive(Deserialize)]
struct Addition {
    document: usize,
    revision: u64,
    requirement: Value,
}

#[derive(Deserialize)]
struct Update {
    key: String,
    revision: u64,
    requirement: Value,
}

#[derive(Deserialize)]
struct Removal {
    key: String,
    revision: u64,
    confirm_id: String,
}

#[derive(Deserialize)]
struct Creation {
    folder: usize,
    filename: String,
    revision: u64,
    document: Value,
    requirement: Value,
}

type ApiError = (StatusCode, Json<Value>);
fn api_error(error: Error) -> ApiError {
    let status = if matches!(error, Error::Conflict(_)) {
        StatusCode::CONFLICT
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    (status, Json(json!({"error":error.to_string()})))
}

async fn data(State(session): State<Session>) -> Result<Json<Value>, ApiError> {
    let mut graph = session.store.lock().snapshot().map_err(api_error)?;
    graph["token"] = json!(session.token);
    Ok(Json(graph))
}

async fn add(
    State(session): State<Session>,
    headers: HeaderMap,
    Json(addition): Json<Addition>,
) -> Result<Json<Value>, ApiError> {
    authorize(&session, &headers)?;
    let token = session.token.clone();
    let mut graph = tokio::task::spawn_blocking(move || {
        session
            .store
            .lock()
            .add(addition.document, addition.revision, addition.requirement)
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Save task failed"})),
        )
    })?
    .map_err(api_error)?;
    graph["token"] = json!(token);
    Ok(Json(graph))
}

async fn update(
    State(session): State<Session>,
    headers: HeaderMap,
    Json(update): Json<Update>,
) -> Result<Json<Value>, ApiError> {
    authorize(&session, &headers)?;
    let token = session.token.clone();
    let mut graph = tokio::task::spawn_blocking(move || {
        session
            .store
            .lock()
            .update(&update.key, update.revision, update.requirement)
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Update task failed"})),
        )
    })?
    .map_err(api_error)?;
    graph["token"] = json!(token);
    Ok(Json(graph))
}

async fn delete(
    State(session): State<Session>,
    headers: HeaderMap,
    Json(removal): Json<Removal>,
) -> Result<Json<Value>, ApiError> {
    authorize(&session, &headers)?;
    let token = session.token.clone();
    let mut graph = tokio::task::spawn_blocking(move || {
        session
            .store
            .lock()
            .delete(&removal.key, removal.revision, &removal.confirm_id)
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Delete task failed"})),
        )
    })?
    .map_err(api_error)?;
    graph["token"] = json!(token);
    Ok(Json(graph))
}

async fn create(
    State(session): State<Session>,
    headers: HeaderMap,
    Json(creation): Json<Creation>,
) -> Result<Json<Value>, ApiError> {
    authorize(&session, &headers)?;
    let token = session.token.clone();
    let mut graph = tokio::task::spawn_blocking(move || {
        session.store.lock().create(
            creation.folder,
            &creation.filename,
            creation.revision,
            creation.document,
            creation.requirement,
        )
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Create task failed"})),
        )
    })?
    .map_err(api_error)?;
    graph["token"] = json!(token);
    Ok(Json(graph))
}

fn authorize(session: &Session, headers: &HeaderMap) -> Result<(), ApiError> {
    let origin = format!("http://{}", session.authority);
    if headers.get("x-viewer-token").and_then(|v| v.to_str().ok()) != Some(&session.token)
        || headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(&origin)
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error":"Invalid session token or origin"})),
        ));
    }
    Ok(())
}

async fn reload(
    State(session): State<Session>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    authorize(&session, &headers)?;
    let token = session.token.clone();
    let mut graph = tokio::task::spawn_blocking(move || session.store.lock().reload())
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"Reload task failed"})),
            )
        })?
        .map_err(api_error)?;
    graph["token"] = json!(token);
    Ok(Json(graph))
}

async fn guard(State(session): State<Session>, request: Request, next: Next) -> Response {
    if request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        != Some(&session.authority)
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("static header"),
    );
    headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; frame-ancestors 'none'".parse().expect("static header"));
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
    response
}

/// Serve embedded assets and session-only APIs; writes require a matching origin/token.
pub fn router(store: Store, authority: String) -> Router {
    let session = Session {
        store: Arc::new(Mutex::new(store)),
        token: uuid::Uuid::new_v4().to_string(),
        authority,
    };
    Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route(
            "/theme.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/theme.js"),
                )
            }),
        )
        .route(
            "/schema-form.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/schema-form.js"),
                )
            }),
        )
        .route("/data", get(data))
        .route(
            "/view.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/view.js"),
                )
            }),
        )
        .route("/documents", axum::routing::post(create))
        .route("/reload", axum::routing::post(reload))
        .route(
            "/requirements",
            axum::routing::post(add).put(update).delete(delete),
        )
        .layer(middleware::from_fn_with_state(session.clone(), guard))
        .with_state(session)
}
