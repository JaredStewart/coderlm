# REPL to API Mapping

This document maps each REPL operation from the CoderLM design (see `PURPOSE.md`) to its corresponding API endpoint. Use this as the reference when building the agent skill that wraps these HTTP calls.

All endpoints are prefixed with `http://<host>:<port>/api/v1`. Examples assume `localhost:3000`.

Every request (except health, session creation, and admin endpoints) **must** include the `X-Session-Id` header. The session ties the request to a specific project.

---

## Session management

Before using any other operation, the agent creates a session **with the working directory** of the project it wants to explore. The server indexes that directory (if not already known) and returns a `session_id` scoped to that project.

| Operation       | Method | Endpoint          | Body / Params | Notes |
|-----------------|--------|-------------------|---------------|-------|
| List sessions   | GET    | `/sessions`       | —             | All active sessions (admin). No session header needed |
| Create session  | POST   | `/sessions`       | `{ "cwd": "/path/to/project" }` | Indexes project if new; returns `{ session_id, created_at, project, structure }` (structure is the L1 file tree, included for free so the agent can orient on session create) |
| Check session   | GET    | `/sessions/:id`   | —             | Returns session info including project path |
| End session     | DELETE | `/sessions/:id`   | —             | Cleans up history |

```bash
# Create — pass the project directory as cwd
SESSION=$(curl -s -X POST localhost:3000/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"cwd":"/home/user/myproject"}' | jq -r .session_id)

# Use on all subsequent requests
curl -H "X-Session-Id: $SESSION" ...
```

If the project was evicted due to capacity limits, requests using that session will return `410 Gone`. Create a new session to re-index.

---

## structure

View the codebase file tree. Equivalent to running `tree` with ignore filtering applied.

| REPL operation           | Method | Endpoint              | Params / Body                                            |
|--------------------------|--------|-----------------------|----------------------------------------------------------|
| `structure`              | GET    | `/structure`          | `?depth=N&detail=L` (depth 0 = unlimited; detail 0–3)    |
| `structure define $file` | POST   | `/structure/define`   | `{ "file": "...", "definition": "..." }`                 |
| `structure redefine $file` | POST | `/structure/redefine` | `{ "file": "...", "definition": "..." }`                 |
| `structure mark $file $type` | POST | `/structure/mark`  | `{ "file": "...", "mark": "..." }`                       |

### Detail levels

`?detail=` controls how much per-file symbol information is returned alongside the tree:

| Level | Contents                                                              |
|-------|-----------------------------------------------------------------------|
| 0     | File tree only (default)                                              |
| 1     | + top-level symbol names, kinds, and signatures per file              |
| 2     | + method signatures and parent/owner info                             |
| 3     | + full source for each symbol                                         |

### Response: `GET /structure` (detail=0)

```json
{
  "tree": "├── src/\n│   ├── main.rs\n│   └── lib.rs\n└── Cargo.toml\n",
  "file_count": 42,
  "language_breakdown": [
    { "language": "rust", "count": 38 },
    { "language": "toml", "count": 4 }
  ]
}
```

### Response: `GET /structure?detail=1`

```json
{
  "tree": "├── src/\n│   ├── main.rs\n│   └── lib.rs\n",
  "file_count": 42,
  "language_breakdown": [{ "language": "rust", "count": 38 }],
  "detail": 1,
  "file_symbols": [
    {
      "file": "src/main.rs",
      "symbols": [
        { "name": "run_server", "kind": "function", "signature": "async fn run_server(", "line": 69 }
      ]
    }
  ]
}
```

### Mark types

`documentation`, `ignore`, `test`, `config`, `generated`, `custom`

### Skill usage pattern

```bash
# 1. Get the structure to orient
curl -s -H "X-Session-Id: $SID" "localhost:3000/api/v1/structure?depth=2"

# 2. Annotate files as the agent learns about them
curl -s -X POST -H "X-Session-Id: $SID" -H "Content-Type: application/json" \
  localhost:3000/api/v1/structure/define \
  -d '{"file":"src/main.rs","definition":"CLI entrypoint, parses args and starts the server"}'

# 3. Mark test directories
curl -s -X POST -H "X-Session-Id: $SID" -H "Content-Type: application/json" \
  localhost:3000/api/v1/structure/mark \
  -d '{"file":"tests/integration.rs","mark":"test"}'
```

---

## symbol list

List symbols extracted from the codebase. Defaults to all kinds; filter with query params.

| REPL operation                  | Method | Endpoint    | Params                                      |
|---------------------------------|--------|-------------|---------------------------------------------|
| `symbol list`                   | GET    | `/symbols`  | `?limit=100`                                |
| `symbol list` (functions only)  | GET    | `/symbols`  | `?kind=function&limit=100`                  |
| `symbol list` (single file)     | GET    | `/symbols`  | `?file=src/main.rs&limit=100`               |
| `symbol list` (combined filter) | GET    | `/symbols`  | `?kind=function&file=src/main.rs&limit=100` |

### Kind values

`function`, `method`, `class`, `struct`, `enum`, `trait`, `interface`, `constant`, `variable`, `type`, `module`

### Response

```json
{
  "count": 3,
  "symbols": [
    {
      "name": "run_server",
      "kind": "function",
      "file": "src/main.rs",
      "line_range": [69, 143],
      "signature": "async fn run_server(",
      "definition": null,
      "parent": null
    }
  ]
}
```

---

## symbol search

Find symbols by name substring. Optionally restrict to a single file.

| REPL operation          | Method | Endpoint          | Params                                |
|-------------------------|--------|-------------------|---------------------------------------|
| `symbol search $query`  | GET    | `/symbols/search` | `?q=handler&limit=20&file=src/main.rs`|

```bash
# Project-wide
curl -s -H "X-Session-Id: $SID" "localhost:3000/api/v1/symbols/search?q=parse&limit=10"

# Restrict to one file
curl -s -H "X-Session-Id: $SID" "localhost:3000/api/v1/symbols/search?q=parse&file=src/main.rs"
```

---

## symbol define / redefine

Annotate a symbol with a human-readable description. Visible to all sessions on the same project.

| REPL operation             | Method | Endpoint            | Body                                                     |
|----------------------------|--------|---------------------|----------------------------------------------------------|
| `symbol define $symbol`    | POST   | `/symbols/define`   | `{ "symbol": "...", "file": "...", "definition": "..." }` |
| `symbol redefine $symbol`  | POST   | `/symbols/redefine` | `{ "symbol": "...", "file": "...", "definition": "..." }` |

`define` fails if a definition already exists (use `redefine` to update). Both require the file path to disambiguate symbols with the same name across files.

```bash
curl -s -X POST -H "X-Session-Id: $SID" -H "Content-Type: application/json" \
  localhost:3000/api/v1/symbols/define \
  -d '{"symbol":"scan_directory","file":"src/index/walker.rs","definition":"Walks codebase respecting gitignore, populates file tree"}'
```

---

## symbol implementation

Retrieve the full source code of a symbol (function body, struct definition, etc.).

| REPL operation                   | Method | Endpoint                  | Params                            |
|----------------------------------|--------|---------------------------|-----------------------------------|
| `symbol implementation $symbol`  | GET    | `/symbols/implementation` | `?symbol=...&file=...`            |

### Response

```json
{
  "symbol": "scan_directory",
  "file": "src/index/walker.rs",
  "source": "pub fn scan_directory(root: &Path, ...) -> Result<usize> {\n    ...\n}"
}
```

---

## symbol callers

Find call sites for a symbol across the codebase.

| REPL operation            | Method | Endpoint          | Params                              |
|---------------------------|--------|-------------------|-------------------------------------|
| `symbol callers $symbol`  | GET    | `/symbols/callers` | `?symbol=...&file=...&limit=50`    |

### Response

```json
{
  "count": 2,
  "callers": [
    { "file": "src/main.rs", "line": 95, "text": "index::walker::scan_directory(" },
    { "file": "src/index/watcher.rs", "line": 133, "text": "extract_symbols_from_file(root, rel_path, language) {" }
  ]
}
```

---

## symbol tests

Find test functions that reference a given symbol.

| REPL operation          | Method | Endpoint         | Params                              |
|-------------------------|--------|------------------|-------------------------------------|
| `symbol tests $symbol`  | GET    | `/symbols/tests` | `?symbol=...&file=...&limit=20`    |

### Response

```json
{
  "count": 1,
  "tests": [
    { "name": "test_scan_directory", "file": "tests/walker_test.rs", "line": 12, "signature": "fn test_scan_directory() {" }
  ]
}
```

---

## symbol list variables

List local variables declared inside a function.

| REPL operation                     | Method | Endpoint             | Params                          |
|------------------------------------|--------|----------------------|---------------------------------|
| `symbol list variables $function`  | GET    | `/symbols/variables` | `?function=...&file=...`        |

### Response

```json
{
  "count": 5,
  "variables": [
    { "name": "walker", "function": "scan_directory" },
    { "name": "count", "function": "scan_directory" },
    { "name": "entry", "function": "scan_directory" }
  ]
}
```

---

## peek

Read a range of lines from a file. Line numbers are 0-indexed (start inclusive, end exclusive).

| REPL operation          | Method | Endpoint | Params                              |
|-------------------------|--------|----------|-------------------------------------|
| `peek $file $start $end`| GET    | `/peek`  | `?file=...&start=0&end=100`         |

### Response

```json
{
  "file": "src/main.rs",
  "start_line": 1,
  "end_line": 10,
  "total_lines": 143,
  "content": "     1 │ mod config;\n     2 │ mod index;\n..."
}
```

### Skill usage pattern

```bash
# Read first 50 lines
curl -s -H "X-Session-Id: $SID" "localhost:3000/api/v1/peek?file=src/main.rs&start=0&end=50"

# Read lines 100-120
curl -s -H "X-Session-Id: $SID" "localhost:3000/api/v1/peek?file=src/main.rs&start=100&end=120"
```

---

## grep

Regex search across all indexed files. Supports scope-aware filtering and per-file restriction.

| REPL operation                  | Method | Endpoint | Params                                                                    |
|---------------------------------|--------|----------|---------------------------------------------------------------------------|
| `grep $pattern`                 | GET    | `/grep`  | `?pattern=...&max_matches=50&context_lines=2&scope=all|code&file=path`    |

### Parameters

| Param           | Default | Notes                                                                  |
|-----------------|---------|------------------------------------------------------------------------|
| `pattern`       | —       | Full Rust regex syntax                                                 |
| `max_matches`   | 50      | Cap on returned matches                                                |
| `context_lines` | 2       | Lines of context before/after each match                               |
| `scope`         | `all`   | `all` matches everywhere; `code` skips matches inside comment/string AST nodes (per-language tree-sitter aware) |
| `file`          | —       | Restrict grep to a single file path                                    |

### Response

```json
{
  "pattern": "DashMap",
  "total_matches": 8,
  "truncated": false,
  "matches": [
    {
      "file": "src/index/file_tree.rs",
      "line": 1,
      "text": "use dashmap::DashMap;",
      "context_before": [],
      "context_after": ["use serde::Serialize;"]
    }
  ]
}
```

```bash
# Skip matches inside comments and string literals
curl -s -H "X-Session-Id: $SID" \
  "localhost:3000/api/v1/grep?pattern=TODO&scope=code"

# Restrict to one file
curl -s -H "X-Session-Id: $SID" \
  "localhost:3000/api/v1/grep?pattern=DashMap&file=src/index/file_tree.rs"
```

---

## chunk_indices

Compute byte-offset chunk boundaries for a file. Useful for splitting large files into pieces for incremental processing.

| REPL operation                          | Method | Endpoint         | Params                                  |
|-----------------------------------------|--------|------------------|-----------------------------------------|
| `chunk_indices $file $size $overlap`    | GET    | `/chunk_indices` | `?file=...&size=5000&overlap=200`       |

### Response

```json
{
  "file": "src/main.rs",
  "total_bytes": 3521,
  "chunk_size": 5000,
  "overlap": 200,
  "chunks": [
    { "index": 0, "start": 0, "end": 3521 }
  ]
}
```

---

## history

Retrieve command history. Supports two modes:

| REPL operation      | Method | Endpoint   | Params         | Session header |
|---------------------|--------|------------|----------------|----------------|
| `history`           | GET    | `/history` | `?limit=50`    | With `X-Session-Id`: single session history |
| `history` (admin)   | GET    | `/history` | `?limit=50`    | Without header: all sessions' history |

### Response (single session)

```json
{
  "count": 3,
  "history": [
    { "timestamp": "2026-02-07T19:01:15Z", "method": "GET", "path": "/structure", "response_preview": "25 files" },
    { "timestamp": "2026-02-07T19:01:18Z", "method": "GET", "path": "/symbols", "response_preview": "42 symbols" },
    { "timestamp": "2026-02-07T19:01:22Z", "method": "GET", "path": "/peek", "response_preview": "src/main.rs:0-50" }
  ]
}
```

### Response (admin — no session header)

```json
{
  "total_entries": 7,
  "sessions": [
    {
      "session_id": "abc-123",
      "project": "/home/user/backend",
      "entries": [
        { "timestamp": "2026-02-07T19:01:15Z", "method": "GET", "path": "/structure", "response_preview": "25 files" }
      ]
    },
    {
      "session_id": "def-456",
      "project": "/home/user/frontend",
      "entries": [
        { "timestamp": "2026-02-07T19:00:55Z", "method": "GET", "path": "/symbols", "response_preview": "42 symbols" }
      ]
    }
  ]
}
```

---

## health

Check server status. Does not require a session.

| Operation | Method | Endpoint  |
|-----------|--------|-----------|
| health    | GET    | `/health` |

```bash
curl -s localhost:3000/api/v1/health
```

### Response

```json
{
  "status": "ok",
  "projects": 2,
  "active_sessions": 3,
  "max_projects": 5
}
```

---

## roots (admin)

List all registered projects. Useful for debugging/admin visibility. Does not require a session.

| Operation | Method | Endpoint  |
|-----------|--------|-----------|
| roots     | GET    | `/roots`  |

```bash
curl -s localhost:3000/api/v1/roots
```

### Response

```json
{
  "count": 2,
  "roots": [
    {
      "path": "/home/user/backend",
      "file_count": 142,
      "symbol_count": 1038,
      "last_active": "2026-02-07T19:05:00Z",
      "session_count": 1
    },
    {
      "path": "/home/user/frontend",
      "file_count": 87,
      "symbol_count": 512,
      "last_active": "2026-02-07T19:03:22Z",
      "session_count": 2
    }
  ]
}
```

---

## annotations (persistence)

Annotations (file definitions, symbol definitions, marks) live in memory by default. These endpoints persist them to / hydrate them from `.coderlm/annotations.json` in the project root. Annotations are also auto-loaded on session creation, and the `Stop` hook auto-saves before cleanup.

| Operation             | Method | Endpoint               | Body | Notes |
|-----------------------|--------|------------------------|------|-------|
| Save annotations      | POST   | `/annotations/save`    | —    | Writes `.coderlm/annotations.json` to the project root |
| Load annotations      | POST   | `/annotations/load`    | —    | Reads `.coderlm/annotations.json` and applies it; returns counts of what was loaded |

```bash
curl -s -X POST -H "X-Session-Id: $SID" localhost:3000/api/v1/annotations/save
# {"ok":true}

curl -s -X POST -H "X-Session-Id: $SID" localhost:3000/api/v1/annotations/load
# {"ok":true,"loaded":{"file_definitions":12,"file_marks":3,"symbol_definitions":47}}
```

---

## buffers

Project-scoped named scratch buffers. Useful for capturing a chunk of source, a snippet under construction, or transient context that should outlive a single request without polluting the file tree. Buffers are in-memory only — they do **not** persist across server restarts.

| Operation                 | Method | Endpoint                          | Body / Params                                                            |
|---------------------------|--------|-----------------------------------|--------------------------------------------------------------------------|
| List buffers              | GET    | `/buffers`                        | —                                                                        |
| Create buffer (raw)       | POST   | `/buffers`                        | `{ "name": "...", "content": "...", "description": "..." }` (description optional) |
| Create buffer from file   | POST   | `/buffers/from-file`              | `{ "name": "...", "file": "...", "start_line": 0, "end_line": 100 }` (start/end optional — full file if omitted) |
| Create buffer from symbol | POST   | `/buffers/from-symbol`            | `{ "name": "...", "symbol": "...", "file": "..." }` (uses tree-sitter byte range) |
| Get buffer (full)         | GET    | `/buffers/{name}`                 | —                                                                        |
| Peek buffer (line range)  | GET    | `/buffers/{name}/peek`            | `?start=0&end=100`                                                       |
| Delete buffer             | DELETE | `/buffers/{name}`                 | —                                                                        |

### Response: `GET /buffers`

```json
{
  "buffers": [
    {
      "name": "scan_dir_snapshot",
      "description": "Symbol scan_directory from src/index/walker.rs",
      "size": 812,
      "lines": 24,
      "created_at": "2026-05-03T12:34:56Z"
    }
  ],
  "count": 1
}
```

### Response: `GET /buffers/{name}`

```json
{
  "name": "scan_dir_snapshot",
  "content": "pub fn scan_directory(...) { ... }",
  "description": "Symbol scan_directory from src/index/walker.rs",
  "created_at": "2026-05-03T12:34:56Z"
}
```

---

## vars

Project-scoped JSON key/value store. Useful for stashing arbitrary intermediate state across requests within a session (e.g. a list of file paths to revisit). Values are arbitrary JSON. In-memory only.

| Operation        | Method | Endpoint        | Body                                    |
|------------------|--------|-----------------|-----------------------------------------|
| List variables   | GET    | `/vars`         | —                                       |
| Set variable     | POST   | `/vars`         | `{ "name": "...", "value": <json> }`    |
| Get variable     | GET    | `/vars/{name}`  | —                                       |
| Delete variable  | DELETE | `/vars/{name}`  | —                                       |

### Response: `GET /vars`

`list_vars` summarises each entry with its type rather than full value:

```json
{
  "variables": [
    { "name": "todo_files", "value_type": "array[7]" },
    { "name": "current_module", "value_type": "string" }
  ],
  "count": 2
}
```

### Response: `GET /vars/{name}`

```json
{ "name": "todo_files", "value": ["src/main.rs", "src/lib.rs"] }
```

---

## subcall_results

Project-scoped store for results from sub-agent calls (used by the recursive exploration pattern, where a parent agent dispatches a question to a child agent and reads back its findings). In-memory only.

| Operation              | Method | Endpoint              | Body                                  |
|------------------------|--------|-----------------------|---------------------------------------|
| List results           | GET    | `/subcall_results`    | —                                     |
| Append result          | POST   | `/subcall_results`    | `SubcallResult` (see below)           |
| Clear all results      | DELETE | `/subcall_results`    | —                                     |

### `SubcallResult` shape

```json
{
  "chunk_id": "src/server/routes.rs:0-300",
  "query": "Where do we register the /annotations/save route?",
  "findings": [
    "build_routes registers POST /api/v1/annotations/save -> save_annotations at routes.rs:87"
  ],
  "suggested_queries": ["What does save_annotations write?"],
  "answer_if_complete": null,
  "depth": 1
}
```

`findings` and `suggested_queries` are free-form string arrays. `answer_if_complete` should be set when the subcall produced a terminal answer (no further hops needed). `depth` records how deep in the recursion the subcall was issued at.

---

## Typical agent workflow

This is the sequence a skill should follow when working with a codebase:

```
1.  GET    /health                            → confirm server is running
2.  POST   /sessions { "cwd": "/path/..." }   → get session_id, project is indexed
                                               (response includes L1 structure for free)
3.  GET    /structure?depth=2&detail=1        → orient: layout + per-file symbol summaries
4.  GET    /symbols?kind=function&limit=50    → scan function inventory
5.  GET    /symbols/search?q=<relevant_term>  → find symbols related to the task
6.  GET    /symbols/implementation?symbol=... → read the source of key functions
7.  GET    /peek?file=...&start=0&end=50      → read file headers / imports
8.  GET    /grep?pattern=<error_msg>&scope=code → locate code patterns (skip comments)
9.  GET    /symbols/callers?symbol=...        → understand how a function is used
10. GET    /symbols/tests?symbol=...          → find existing test coverage
11. POST   /structure/define                  → annotate files as understanding grows
12. POST   /symbols/define                    → annotate symbols
13. POST   /annotations/save                  → persist annotations to disk
14. GET    /history                           → review what has been explored
15. DELETE /sessions/:id                      → clean up when done
```

Steps 3-13 repeat as needed. The agent builds up a mental map of the codebase incrementally, annotating as it goes so that subsequent queries (by the same agent or by other agents in a swarm) benefit from the accumulated definitions.

---

## Multi-project setup

A single server instance supports multiple projects simultaneously. Each project is indexed on-demand when an agent creates a session with that project's `cwd`. There is no need to run separate server instances per repo.

```bash
# Start the server (no project path required)
coderlm-server serve --port 3000

# Agent A connects to the backend
curl -X POST localhost:3000/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"cwd":"/home/user/backend"}'

# Agent B connects to the frontend
curl -X POST localhost:3000/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"cwd":"/home/user/frontend"}'
```

Each session is scoped to its project — queries from Agent A only see backend files/symbols, and queries from Agent B only see frontend files/symbols. Annotations (definitions, marks) set by one session are visible to all sessions on the **same project**.

### Capacity and eviction

The server maintains at most `--max-projects` indexed projects (default: 5). When a new project would exceed this limit, the least recently used project is evicted — its file tree, symbols, and watcher are dropped, and any sessions still pointing to it will receive `410 Gone` responses. Those agents can simply create a new session to re-index.

```bash
# Allow up to 10 concurrent projects
coderlm-server serve --max-projects 10

# Pre-index a project at startup (optional)
coderlm-server serve /home/user/main-project --max-projects 5
```
