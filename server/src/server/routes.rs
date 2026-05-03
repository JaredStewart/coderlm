use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::ops::{annotations, buffers, content, history, structure, subcalls, symbol_ops, variables};
use crate::server::errors::AppError;
use crate::server::session::Session;
use crate::server::state::{AppState, Project};
use crate::symbols::symbol::SymbolKind;

// ---------------------------------------------------------------------------
// Helper: extract session ID from headers
// ---------------------------------------------------------------------------

fn session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn require_session(headers: &HeaderMap) -> Result<String, AppError> {
    session_id(headers).ok_or_else(|| AppError::BadRequest("Missing X-Session-Id header".into()))
}

/// Resolve session -> project. Touches last_active on both session and project.
fn require_project(state: &AppState, headers: &HeaderMap) -> Result<Arc<Project>, AppError> {
    let sid = require_session(headers)?;
    let project = state.get_project_for_session(&sid)?;
    state.touch_project(&project.root);
    // Update session last_active
    if let Some(mut session) = state.inner.sessions.get_mut(&sid) {
        session.last_active = chrono::Utc::now();
    }
    Ok(project)
}

fn record_history(state: &AppState, session_id: Option<&str>, method: &str, path: &str, preview: &str) {
    if let Some(id) = session_id {
        if let Some(mut session) = state.inner.sessions.get_mut(id) {
            session.record(method, path, preview);
        }
    }
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

pub fn build_routes(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/api/v1/health", get(health))
        // Admin
        .route("/api/v1/roots", get(list_roots))
        // Sessions
        .route("/api/v1/sessions", get(list_sessions).post(create_session))
        .route("/api/v1/sessions/{id}", get(get_session))
        .route("/api/v1/sessions/{id}", delete(delete_session))
        // Structure
        .route("/api/v1/structure", get(get_structure))
        .route("/api/v1/structure/define", post(define_file))
        .route("/api/v1/structure/redefine", post(redefine_file))
        .route("/api/v1/structure/mark", post(mark_file))
        // Symbols
        .route("/api/v1/symbols", get(list_symbols))
        .route("/api/v1/symbols/search", get(search_symbols))
        .route("/api/v1/symbols/define", post(define_symbol))
        .route("/api/v1/symbols/redefine", post(redefine_symbol))
        .route("/api/v1/symbols/implementation", get(get_implementation))
        .route("/api/v1/symbols/tests", get(find_tests))
        .route("/api/v1/symbols/callers", get(find_callers))
        .route("/api/v1/symbols/variables", get(list_variables))
        // Content
        .route("/api/v1/peek", get(peek))
        .route("/api/v1/grep", get(grep_handler))
        .route("/api/v1/chunk_indices", get(chunk_indices))
        // History
        .route("/api/v1/history", get(get_history))
        // Annotations
        .route("/api/v1/annotations/save", post(save_annotations))
        .route("/api/v1/annotations/load", post(load_annotations))
        // Buffers
        .route("/api/v1/buffers", get(list_buffers).post(create_buffer))
        .route("/api/v1/buffers/from-file", post(buffer_from_file))
        .route("/api/v1/buffers/from-symbol", post(buffer_from_symbol))
        .route("/api/v1/buffers/{name}", get(get_buffer).delete(delete_buffer))
        .route("/api/v1/buffers/{name}/peek", get(peek_buffer))
        // Variables
        .route("/api/v1/vars", get(list_vars).post(set_var))
        .route("/api/v1/vars/{name}", get(get_var).delete(delete_var))
        // Subcall results
        .route("/api/v1/subcall_results", get(get_subcall_results).post(store_subcall_result).delete(clear_subcall_results))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

async fn health(State(state): State<AppState>) -> Json<Value> {
    let project_count = state.inner.projects.len();
    let session_count = state.inner.sessions.len();

    Json(json!({
        "status": "ok",
        "projects": project_count,
        "active_sessions": session_count,
        "max_projects": state.inner.max_projects,
    }))
}

// ---------------------------------------------------------------------------
// Admin: list registered projects
// ---------------------------------------------------------------------------

async fn list_roots(State(state): State<AppState>) -> Json<Value> {
    let roots: Vec<Value> = state
        .inner
        .projects
        .iter()
        .map(|entry| {
            let project = entry.value();
            let session_count = state
                .inner
                .sessions
                .iter()
                .filter(|s| s.value().project_path == *entry.key())
                .count();
            json!({
                "path": project.root.display().to_string(),
                "file_count": project.file_tree.len(),
                "symbol_count": project.symbol_table.len(),
                "last_active": (*project.last_active.lock()).to_rfc3339(),
                "session_count": session_count,
            })
        })
        .collect();

    Json(json!({ "roots": roots, "count": roots.len() }))
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateSessionBody {
    cwd: String,
}

async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionBody>,
) -> Result<Json<Value>, AppError> {
    let cwd_path = PathBuf::from(&body.cwd);

    // Index the project (or return existing)
    let project = state.get_or_create_project(&cwd_path)?;

    let id = uuid::Uuid::new_v4().to_string();
    let session = Session::new(id.clone(), project.root.clone());
    let created_at = session.created_at;
    state.inner.sessions.insert(id.clone(), session);

    // Load annotations from disk after project is indexed
    let ft = project.file_tree.clone();
    let st = project.symbol_table.clone();
    let root = project.root.clone();
    tokio::spawn(async move {
        // Small delay to let symbol extraction start first
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let _ = annotations::load_annotations(&root, &ft, &st);
    });

    // Return L1 structure in the session creation response
    let structure_result = structure::get_structure_with_detail(
        &project.root,
        &project.file_tree,
        &project.symbol_table,
        0,
        1,
    );

    Ok(Json(json!({
        "session_id": id,
        "created_at": created_at.to_rfc3339(),
        "project": project.root.display().to_string(),
        "structure": serde_json::to_value(&structure_result).unwrap_or(json!(null)),
    })))
}

#[derive(Deserialize)]
struct SessionPath {
    id: String,
}

async fn get_session(
    State(state): State<AppState>,
    axum::extract::Path(params): axum::extract::Path<SessionPath>,
) -> Result<Json<Value>, AppError> {
    let session = state
        .inner
        .sessions
        .get(&params.id)
        .ok_or_else(|| AppError::NotFound(format!("Session '{}' not found", params.id)))?;

    Ok(Json(json!({
        "session_id": session.id,
        "project": session.project_path.display().to_string(),
        "created_at": session.created_at.to_rfc3339(),
        "last_active": session.last_active.to_rfc3339(),
        "history_count": session.history.len(),
    })))
}

async fn delete_session(
    State(state): State<AppState>,
    axum::extract::Path(params): axum::extract::Path<SessionPath>,
) -> Result<Json<Value>, AppError> {
    state
        .inner
        .sessions
        .remove(&params.id)
        .ok_or_else(|| AppError::NotFound(format!("Session '{}' not found", params.id)))?;

    Ok(Json(json!({ "deleted": true })))
}

async fn list_sessions(State(state): State<AppState>) -> Json<Value> {
    let mut sessions: Vec<Value> = state
        .inner
        .sessions
        .iter()
        .map(|entry| {
            let session = entry.value();
            json!({
                "session_id": session.id,
                "project": session.project_path.display().to_string(),
                "created_at": session.created_at.to_rfc3339(),
                "last_active": session.last_active.to_rfc3339(),
                "history_count": session.history.len(),
            })
        })
        .collect();

    sessions.sort_by(|a, b| {
        let a_time = a["last_active"].as_str().unwrap_or("");
        let b_time = b["last_active"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });

    Json(json!({ "sessions": sessions, "count": sessions.len() }))
}

// ---------------------------------------------------------------------------
// Structure
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct StructureQuery {
    depth: Option<usize>,
    /// Level-of-detail: 0=tree only, 1=+symbols, 2=+signatures, 3=+source
    detail: Option<u8>,
}

async fn get_structure(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<StructureQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let depth = params.depth.unwrap_or(0);
    let detail = params.detail.unwrap_or(0);

    if detail > 0 {
        let result = structure::get_structure_with_detail(
            &project.root,
            &project.file_tree,
            &project.symbol_table,
            depth,
            detail,
        );
        let preview = format!("{} files (L{})", result.file_count, detail);
        record_history(&state, session_id(&headers).as_deref(), "GET", "/structure", &preview);
        Ok(Json(serde_json::to_value(result).unwrap()))
    } else {
        let result = structure::get_structure(&project.file_tree, depth);
        let preview = format!("{} files", result.file_count);
        record_history(&state, session_id(&headers).as_deref(), "GET", "/structure", &preview);
        Ok(Json(serde_json::to_value(result).unwrap()))
    }
}

#[derive(Deserialize)]
struct DefineRequest {
    file: String,
    definition: String,
}

async fn define_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DefineRequest>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    structure::define_file(&project.file_tree, &body.file, &body.definition)
        .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/structure/define", &body.file);
    Ok(Json(json!({ "ok": true })))
}

async fn redefine_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DefineRequest>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    structure::redefine_file(&project.file_tree, &body.file, &body.definition)
        .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/structure/redefine", &body.file);
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct MarkRequest {
    file: String,
    mark: String,
}

async fn mark_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MarkRequest>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    structure::mark_file(&project.file_tree, &body.file, &body.mark)
        .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/structure/mark", &body.file);
    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Symbols
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SymbolListQuery {
    kind: Option<String>,
    file: Option<String>,
    limit: Option<usize>,
}

async fn list_symbols(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SymbolListQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let kind_filter = params.kind.as_deref().and_then(SymbolKind::from_str);
    let limit = params.limit.unwrap_or(100);
    let results = symbol_ops::list_symbols(
        &project.symbol_table,
        kind_filter,
        params.file.as_deref(),
        limit,
    );
    let preview = format!("{} symbols", results.len());
    record_history(&state, session_id(&headers).as_deref(), "GET", "/symbols", &preview);
    Ok(Json(json!({ "symbols": results, "count": results.len() })))
}

#[derive(Deserialize)]
struct SymbolSearchQuery {
    q: String,
    limit: Option<usize>,
    file: Option<String>,
}

async fn search_symbols(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SymbolSearchQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let limit = params.limit.unwrap_or(20);
    let results = symbol_ops::search_symbols(&project.symbol_table, &params.q, limit, params.file.as_deref());
    let preview = format!("{} matches for '{}'", results.len(), params.q);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/symbols/search", &preview);
    Ok(Json(json!({ "symbols": results, "count": results.len() })))
}

#[derive(Deserialize)]
struct SymbolDefineRequest {
    symbol: String,
    file: String,
    definition: String,
}

async fn define_symbol(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SymbolDefineRequest>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    symbol_ops::define_symbol(
        &project.symbol_table,
        &body.symbol,
        &body.file,
        &body.definition,
    )
    .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/symbols/define", &body.symbol);
    Ok(Json(json!({ "ok": true })))
}

async fn redefine_symbol(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SymbolDefineRequest>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    symbol_ops::redefine_symbol(
        &project.symbol_table,
        &body.symbol,
        &body.file,
        &body.definition,
    )
    .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/symbols/redefine", &body.symbol);
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ImplementationQuery {
    symbol: String,
    file: String,
}

async fn get_implementation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ImplementationQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let source = symbol_ops::get_implementation(
        &project.root,
        &project.symbol_table,
        &params.symbol,
        &params.file,
    )
    .map_err(AppError::NotFound)?;
    let preview = format!("{}::{} ({} bytes)", params.file, params.symbol, source.len());
    record_history(&state, session_id(&headers).as_deref(), "GET", "/symbols/implementation", &preview);
    Ok(Json(json!({
        "symbol": params.symbol,
        "file": params.file,
        "source": source,
    })))
}

#[derive(Deserialize)]
struct TestsQuery {
    symbol: String,
    file: String,
    limit: Option<usize>,
}

async fn find_tests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TestsQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let limit = params.limit.unwrap_or(20);
    let tests = symbol_ops::find_tests(
        &project.root,
        &project.file_tree,
        &project.symbol_table,
        &params.symbol,
        &params.file,
        limit,
    )
    .map_err(AppError::NotFound)?;
    let preview = format!("{} tests for {}", tests.len(), params.symbol);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/symbols/tests", &preview);
    Ok(Json(json!({ "tests": tests, "count": tests.len() })))
}

#[derive(Deserialize)]
struct CallersQuery {
    symbol: String,
    file: String,
    limit: Option<usize>,
}

async fn find_callers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<CallersQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let limit = params.limit.unwrap_or(50);
    let callers = symbol_ops::find_callers(
        &project.root,
        &project.file_tree,
        &project.symbol_table,
        &params.symbol,
        &params.file,
        limit,
    )
    .map_err(AppError::NotFound)?;
    let preview = format!("{} callers of {}", callers.len(), params.symbol);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/symbols/callers", &preview);
    Ok(Json(json!({ "callers": callers, "count": callers.len() })))
}

#[derive(Deserialize)]
struct VariablesQuery {
    function: String,
    file: String,
}

async fn list_variables(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<VariablesQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let vars = symbol_ops::list_variables(
        &project.root,
        &project.symbol_table,
        &params.function,
        &params.file,
    )
    .map_err(AppError::NotFound)?;
    let preview = format!("{} variables in {}", vars.len(), params.function);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/symbols/variables", &preview);
    Ok(Json(json!({ "variables": vars, "count": vars.len() })))
}

// ---------------------------------------------------------------------------
// Content
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PeekQuery {
    file: String,
    start: Option<usize>,
    end: Option<usize>,
}

async fn peek(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<PeekQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let start = params.start.unwrap_or(0);
    let end = params.end.unwrap_or(100);
    let result = content::peek(
        &project.root,
        &project.file_tree,
        &params.file,
        start,
        end,
    )
    .map_err(AppError::NotFound)?;
    let preview = format!("{}:{}-{}", params.file, start, end);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/peek", &preview);
    Ok(Json(serde_json::to_value(result).unwrap()))
}

#[derive(Deserialize)]
struct GrepQuery {
    pattern: String,
    max_matches: Option<usize>,
    context_lines: Option<usize>,
    /// Optional scope filter: "all" (default) or "code" (skip comments/strings).
    scope: Option<String>,
    /// Optional file path filter to restrict grep to matching files.
    file: Option<String>,
}

async fn grep_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<GrepQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let max_matches = params.max_matches.unwrap_or(50);
    let context_lines = params.context_lines.unwrap_or(2);
    let scope = params
        .scope
        .as_deref()
        .map(|s| content::GrepScope::from_str(s))
        .flatten()
        .unwrap_or(content::GrepScope::All);

    // Run grep on a blocking thread since it reads many files
    let root = project.root.clone();
    let file_tree = project.file_tree.clone();
    let pattern = params.pattern.clone();
    let file_filter = params.file.clone();

    let result = tokio::task::spawn_blocking(move || {
        content::grep_with_scope(&root, &file_tree, &pattern, max_matches, context_lines, scope, file_filter.as_deref())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .map_err(AppError::BadRequest)?;

    let preview = format!("{} matches for '{}'", result.total_matches, params.pattern);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/grep", &preview);
    Ok(Json(serde_json::to_value(result).unwrap()))
}

#[derive(Deserialize)]
struct ChunkQuery {
    file: String,
    size: Option<usize>,
    overlap: Option<usize>,
}

async fn chunk_indices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ChunkQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let size = params.size.unwrap_or(5000);
    let overlap = params.overlap.unwrap_or(200);
    let result = content::chunk_indices(
        &project.root,
        &project.file_tree,
        &params.file,
        size,
        overlap,
    )
    .map_err(AppError::BadRequest)?;
    let preview = format!("{} chunks for {}", result.chunks.len(), params.file);
    record_history(&state, session_id(&headers).as_deref(), "GET", "/chunk_indices", &preview);
    Ok(Json(serde_json::to_value(result).unwrap()))
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
}

async fn get_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Value>, AppError> {
    let limit = params.limit.unwrap_or(50);

    // If no session header, return history from all active sessions (admin view)
    match session_id(&headers) {
        Some(sid) => {
            let _project = state.get_project_for_session(&sid)?;
            let entries =
                history::get_history(&state, &sid, limit).map_err(AppError::NotFound)?;
            Ok(Json(json!({ "history": entries, "count": entries.len() })))
        }
        None => {
            let blocks = history::get_all_history(&state, limit);
            let total: usize = blocks.iter().map(|b| b.entries.len()).sum();
            Ok(Json(json!({ "sessions": blocks, "total_entries": total })))
        }
    }
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

async fn save_annotations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    annotations::save_annotations(&project.root, &project.file_tree, &project.symbol_table)
        .map_err(AppError::Internal)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/annotations/save", "saved");
    Ok(Json(json!({ "ok": true })))
}

async fn load_annotations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let data = annotations::load_annotations(
        &project.root,
        &project.file_tree,
        &project.symbol_table,
    )
    .map_err(AppError::Internal)?;
    let summary = json!({
        "file_definitions": data.file_definitions.len(),
        "file_marks": data.file_marks.len(),
        "symbol_definitions": data.symbol_definitions.len(),
    });
    record_history(&state, session_id(&headers).as_deref(), "POST", "/annotations/load", "loaded");
    Ok(Json(json!({ "ok": true, "loaded": summary })))
}

// ---------------------------------------------------------------------------
// Buffers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateBufferBody {
    name: String,
    content: String,
    description: Option<String>,
}

async fn create_buffer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBufferBody>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let buf = buffers::create_buffer(
        &project.buffers,
        &body.name,
        &body.content,
        body.description.as_deref(),
    )
    .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/buffers", &body.name);
    Ok(Json(json!({ "ok": true, "buffer": buf.name, "size": buf.content.len() })))
}

#[derive(Deserialize)]
struct BufferFromFileBody {
    name: String,
    file: String,
    start_line: Option<usize>,
    end_line: Option<usize>,
}

async fn buffer_from_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BufferFromFileBody>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let buf = buffers::from_file(
        &project.buffers,
        &project.root,
        &project.file_tree,
        &body.name,
        &body.file,
        body.start_line,
        body.end_line,
    )
    .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/buffers/from-file", &body.name);
    Ok(Json(json!({ "ok": true, "buffer": buf.name, "size": buf.content.len() })))
}

#[derive(Deserialize)]
struct BufferFromSymbolBody {
    name: String,
    symbol: String,
    file: String,
}

async fn buffer_from_symbol(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BufferFromSymbolBody>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let buf = buffers::from_symbol(
        &project.buffers,
        &project.root,
        &project.symbol_table,
        &body.name,
        &body.symbol,
        &body.file,
    )
    .map_err(AppError::BadRequest)?;
    record_history(&state, session_id(&headers).as_deref(), "POST", "/buffers/from-symbol", &body.name);
    Ok(Json(json!({ "ok": true, "buffer": buf.name, "size": buf.content.len() })))
}

async fn list_buffers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let list = buffers::list_buffers(&project.buffers);
    Ok(Json(json!({ "buffers": list, "count": list.len() })))
}

#[derive(Deserialize)]
struct BufferPath {
    name: String,
}

async fn get_buffer(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(params): axum::extract::Path<BufferPath>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let buf = buffers::get_buffer(&project.buffers, &params.name)
        .map_err(AppError::NotFound)?;
    Ok(Json(serde_json::to_value(buf).unwrap()))
}

async fn delete_buffer(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(params): axum::extract::Path<BufferPath>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    buffers::delete_buffer(&project.buffers, &params.name)
        .map_err(AppError::NotFound)?;
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
struct PeekBufferQuery {
    start: Option<usize>,
    end: Option<usize>,
}

async fn peek_buffer(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(params): axum::extract::Path<BufferPath>,
    Query(query): Query<PeekBufferQuery>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let start = query.start.unwrap_or(0);
    let end = query.end.unwrap_or(100);
    let content = buffers::peek_buffer(&project.buffers, &params.name, start, end)
        .map_err(AppError::NotFound)?;
    Ok(Json(json!({ "buffer": params.name, "content": content })))
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SetVarBody {
    name: String,
    value: serde_json::Value,
}

async fn set_var(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SetVarBody>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    variables::set_var(&project.variables, &body.name, body.value);
    record_history(&state, session_id(&headers).as_deref(), "POST", "/vars", &body.name);
    Ok(Json(json!({ "ok": true })))
}

async fn list_vars(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let list = variables::list_vars(&project.variables);
    Ok(Json(json!({ "variables": list, "count": list.len() })))
}

#[derive(Deserialize)]
struct VarPath {
    name: String,
}

async fn get_var(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(params): axum::extract::Path<VarPath>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let val = variables::get_var(&project.variables, &params.name)
        .map_err(AppError::NotFound)?;
    Ok(Json(json!({ "name": params.name, "value": val })))
}

async fn delete_var(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(params): axum::extract::Path<VarPath>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    variables::delete_var(&project.variables, &params.name)
        .map_err(AppError::NotFound)?;
    Ok(Json(json!({ "deleted": true })))
}

// ---------------------------------------------------------------------------
// Subcall Results
// ---------------------------------------------------------------------------

async fn store_subcall_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<subcalls::SubcallResult>,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    subcalls::store_result(&project.subcall_results, body);
    Ok(Json(json!({ "ok": true })))
}

async fn get_subcall_results(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    let results = subcalls::get_results(&project.subcall_results);
    Ok(Json(json!({ "results": results, "count": results.len() })))
}

async fn clear_subcall_results(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let project = require_project(&state, &headers)?;
    subcalls::clear_results(&project.subcall_results);
    Ok(Json(json!({ "cleared": true })))
}
