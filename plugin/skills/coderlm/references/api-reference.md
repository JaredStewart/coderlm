# CoderLM Server API Reference

All endpoints prefixed with `/api/v1`. Session-scoped endpoints require `X-Session-Id` header.
The CLI wrapper (`coderlm_cli.py`) handles headers and session management automatically.

## Modes of Invocation

The CLI supports three styles of invocation:

1. **Single command** (one HTTP call per CLI invocation):
   ```bash
   python3 cli search MyFunction
   ```
2. **Batch CLI commands** (newline-separated; each line is parsed as a CLI subcommand and run in order, sharing the same session):
   ```bash
   python3 cli batch --commands "
     search MyFunction
     impl MyFunction --file path/to/file.py
     callers MyFunction --file path/to/file.py
   "
   ```
3. **Python exec** (run Python with helper functions in scope — lets later queries depend on earlier results without spawning new subprocesses):
   ```bash
   python3 cli exec --code "
     hits = search('serialize_response')
     if hits.get('symbols'):
         sym = hits['symbols'][0]
         impl_(sym['name'], sym['file'])
         callers(sym['name'], sym['file'])
   "
   ```

   Helpers available inside `exec`: `search()`, `impl_()`, `callers()`, `tests()`, `grep()`, `symbols()`, `peek_file()`, `structure()`, `variables_list()`, plus the `json` module. Each helper prints the JSON response and also returns it for use in subsequent code.

Use **batch** for 2–5 known lookups. Use **exec** when later queries depend on earlier results, when tracing 3+ hops, or when you want a full trace returned in a single tool call.

## CLI Command Reference

All commands below assume the CLI is at `skills/coderlm/scripts/coderlm_cli.py`.
Abbreviated as `cli` below.

### Session Management

```bash
# Create session (indexes project, caches session ID)
python3 cli init [--cwd /path/to/project] [--port 3000]

# Server + session status
python3 cli status

# Delete session
python3 cli cleanup
```

### Codebase Structure

```bash
# File tree (depth 0 = unlimited)
python3 cli structure [--depth 2]

# Annotate a file
python3 cli define-file src/main.rs "CLI entrypoint, parses args and starts server"
python3 cli redefine-file src/main.rs "Updated description"

# Tag file type: documentation, ignore, test, config, generated, custom
python3 cli mark tests/integration.rs test
```

The server's `/structure` endpoint accepts a `detail` query param (0=tree only, 1=+symbol summaries, 2=+method/parent info, 3=+full source) — the `init` response also includes the L1 structure for free, so the agent gets a per-file symbol overview without an extra call.

### Symbol Operations

```bash
# List symbols (filter by kind, file, or both)
python3 cli symbols [--kind function] [--file src/main.rs] [--limit 50]

# Search symbols by name substring (optionally restrict to one file)
python3 cli search "handler" [--limit 20] [--file src/main.rs]

# Get full source code of a symbol
python3 cli impl run_server --file src/main.rs

# Find call sites
python3 cli callers scan_directory --file src/index/walker.rs [--limit 50]

# Find tests referencing a symbol
python3 cli tests scan_directory --file src/index/walker.rs [--limit 20]

# List local variables in a function
python3 cli variables scan_directory --file src/index/walker.rs

# Annotate a symbol
python3 cli define-symbol scan_directory --file src/index/walker.rs "Walks codebase respecting gitignore"
python3 cli redefine-symbol scan_directory --file src/index/walker.rs "Updated description"
```

### Content Operations

```bash
# Read lines from a file (0-indexed, end exclusive)
python3 cli peek src/main.rs [--start 0] [--end 50]

# Regex search across all indexed files
python3 cli grep "DashMap" [--max-matches 50] [--context-lines 2]

# Scope-aware grep: only match in code (skip comments and strings)
python3 cli grep "DashMap" --scope code

# Restrict grep to one file
python3 cli grep "DashMap" --file src/index/file_tree.rs

# Compute byte-range chunks for a file
python3 cli chunks src/main.rs [--size 5000] [--overlap 200]
```

### Batch and Exec Modes

```bash
# Batch: run several CLI commands in one invocation
python3 cli batch --commands "
  search MyFunction
  impl MyFunction --file path/to/file.py
  callers MyFunction --file path/to/file.py
"

# Exec: programmatic, multi-hop traces in a single call
python3 cli exec --code "
  hits = search('serialize_response')
  if hits.get('symbols'):
      sym = hits['symbols'][0]
      impl_(sym['name'], sym['file'])
      callers(sym['name'], sym['file'])
"
```

### Annotations

```bash
# Save annotations (definitions + marks) to .coderlm/annotations.json
python3 cli save-annotations

# Load annotations from disk (auto-loaded on session creation)
python3 cli load-annotations
```

### History

```bash
# Session command history
python3 cli history [--limit 50]
```

## Server-only Endpoints (no CLI wrapper)

The following endpoints exist on the server but are not yet wrapped in `coderlm_cli.py`. They can be invoked with `curl` against `http://127.0.0.1:3000/api/v1/...` using the `X-Session-Id` header. See `server/REPL_to_API.md` for full request/response details.

### Buffers (`/buffers`)

Project-scoped, in-memory named scratch buffers.

| Method | Endpoint                        | Body / Params                                                  |
|--------|---------------------------------|----------------------------------------------------------------|
| GET    | `/buffers`                      | —                                                              |
| POST   | `/buffers`                      | `{ "name", "content", "description"? }`                        |
| POST   | `/buffers/from-file`            | `{ "name", "file", "start_line"?, "end_line"? }`               |
| POST   | `/buffers/from-symbol`          | `{ "name", "symbol", "file" }`                                 |
| GET    | `/buffers/{name}`               | —                                                              |
| GET    | `/buffers/{name}/peek`          | `?start=N&end=N`                                               |
| DELETE | `/buffers/{name}`               | —                                                              |

### Vars (`/vars`)

Project-scoped, in-memory JSON key/value store.

| Method | Endpoint        | Body                              |
|--------|-----------------|-----------------------------------|
| GET    | `/vars`         | — (returns `{name, value_type}` summaries) |
| POST   | `/vars`         | `{ "name", "value": <json> }`     |
| GET    | `/vars/{name}`  | —                                 |
| DELETE | `/vars/{name}`  | —                                 |

### Subcall Results (`/subcall_results`)

Project-scoped store for findings from sub-agent exploration calls.

| Method | Endpoint            | Body                                  |
|--------|---------------------|---------------------------------------|
| GET    | `/subcall_results`  | —                                     |
| POST   | `/subcall_results`  | `SubcallResult` JSON (see REPL_to_API.md) |
| DELETE | `/subcall_results`  | —                                     |

## Response Shapes

### structure
```json
{
  "tree": "├── src/\n│   ├── main.rs\n...",
  "file_count": 42,
  "language_breakdown": [{"language": "rust", "count": 38}]
}
```

### symbols
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

### search
Same shape as symbols response.

### impl
```json
{
  "symbol": "scan_directory",
  "file": "src/index/walker.rs",
  "source": "pub fn scan_directory(root: &Path) -> Result<usize> {\n    ...\n}"
}
```

### callers
```json
{
  "count": 2,
  "callers": [
    {"file": "src/main.rs", "line": 95, "text": "walker::scan_directory("}
  ]
}
```

### tests
```json
{
  "count": 1,
  "tests": [
    {"name": "test_scan_directory", "file": "tests/walker_test.rs", "line": 12, "signature": "fn test_scan_directory() {"}
  ]
}
```

### variables
```json
{
  "count": 3,
  "variables": [
    {"name": "walker", "function": "scan_directory"},
    {"name": "count", "function": "scan_directory"}
  ]
}
```

### peek
```json
{
  "file": "src/main.rs",
  "start_line": 1,
  "end_line": 10,
  "total_lines": 143,
  "content": "     1 │ mod config;\n     2 │ mod index;\n..."
}
```

### grep
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

### chunks
```json
{
  "file": "src/main.rs",
  "total_bytes": 3521,
  "chunk_size": 5000,
  "overlap": 200,
  "chunks": [{"index": 0, "start": 0, "end": 3521}]
}
```

### health
```json
{
  "status": "ok",
  "projects": 2,
  "active_sessions": 3,
  "max_projects": 5
}
```

## Symbol Kinds

`function`, `method`, `class`, `struct`, `enum`, `trait`, `interface`, `constant`, `variable`, `type`, `module`

## Supported Languages (tree-sitter)

| Language   | Extensions                    | Support      |
|------------|-------------------------------|--------------|
| Rust       | `.rs`                         | tree-sitter  |
| Python     | `.py`, `.pyi`                 | tree-sitter  |
| TypeScript | `.ts`, `.tsx`                 | tree-sitter  |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | tree-sitter  |
| Go         | `.go`                         | tree-sitter  |
| Java       | `.java`                       | tree-sitter  |
| Scala      | `.scala`, `.sc`               | tree-sitter  |
| Ruby       | `.rb`, `.rake`                | tree-sitter  |
| PHP        | `.php`, `.phtml`              | tree-sitter  |
| Zig        | `.zig`, `.zon`                | tree-sitter  |
| SQL        | `.sql`                        | regex        |

Languages with tree-sitter support produce full symbol tables (functions, classes, methods, callers, variables). SQL uses regex fallbacks for variable and definition detection. All other file types appear in the file tree and are searchable via peek/grep, but do not produce symbols.

## Mark Types

`documentation`, `ignore`, `test`, `config`, `generated`, `custom`

## Error Codes

| Status | Meaning |
|--------|---------|
| 400    | Bad request (missing/invalid parameters) |
| 404    | Resource not found |
| 410    | Project evicted — create a new session |
| 500    | Server error |
