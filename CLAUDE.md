# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

CodeRLM applies the Recursive Language Model (RLM) pattern to codebases. A Rust server indexes a project's files and symbols (via tree-sitter), then exposes a JSON API that LLM agents query for targeted context — structure, symbols, source, callers, tests, grep. An agent skill (`plugin/skills/coderlm/`) wraps the API with a Python CLI and a structured workflow so Claude Code can explore unfamiliar codebases without loading everything into context.

## Repository Layout

- `server/` — Rust codebase (the only code that gets built). Has its own `.git`.
- `plugin/` — Self-contained Claude Code plugin root
  - `plugin/skills/coderlm/` — Skill definition + Python CLI wrapper
  - `plugin/hooks/` — Claude Code hooks (SessionStart, UserPromptSubmit, PreCompact, Stop)
  - `plugin/commands/` — Slash command definitions
  - `plugin/scripts/` — Hook scripts (session lifecycle)
  - `plugin/.claude-plugin/` — Plugin manifest (`plugin.json`)
- `.claude-plugin/` — Marketplace manifest (`marketplace.json`), points to `plugin/`

## Build & Run

All commands run from `server/`:

```bash
# Build
cd server && cargo build

# Build release
cd server && cargo build --release

# Run
cd server && cargo run -- serve                       # no project pre-indexed
cd server && cargo run -- serve /path/to/project      # pre-index a project
cd server && cargo run -- serve --port 8080           # custom port

# Check compilation without building binary
cd server && cargo check

# Verify server is running
curl http://127.0.0.1:3000/api/v1/health
```

There are no tests yet. When adding tests, use `cargo test` from `server/`.

## Server Architecture

The server is a single-binary axum application. Key modules under `server/src/`:

- **`main.rs`** — CLI parsing (clap) and server startup
- **`server/`** — HTTP layer
  - `state.rs` — `AppState` holding `DashMap<PathBuf, Project>` and `DashMap<String, Session>`. Multi-project support with LRU eviction at capacity.
  - `routes.rs` — All route handlers. Each handler calls `require_project()` to resolve session→project, then delegates to an `ops` function.
  - `session.rs` — Session struct with command history
  - `errors.rs` — `AppError` enum mapped to HTTP status codes (400/404/410/500)
- **`index/`** — Filesystem indexing
  - `file_tree.rs` — `FileTree` (DashMap of relative path → `FileEntry`). Supports definitions and marks.
  - `walker.rs` — Gitignore-aware directory scan using the `ignore` crate
  - `watcher.rs` — `notify-debouncer-mini` filesystem watcher; re-indexes changed files
  - `file_entry.rs` — `FileEntry` struct (path, size, language, definition, mark)
- **`symbols/`** — Symbol extraction
  - `mod.rs` — `SymbolTable` with DashMap primary store keyed by `"file::name"` and secondary indices by name and file
  - `symbol.rs` — `Symbol` struct and `SymbolKind` enum
  - `parser.rs` — Runs tree-sitter on files, dispatches to per-language queries
  - `queries/` — Tree-sitter query strings per language: symbols, callers (call-expression), variables (local bindings) for Rust, Python, TypeScript, Go (JS reuses TS base)
- **`ops/`** — Business logic called by route handlers
  - `structure.rs` — File tree rendering, define/redefine/mark
  - `symbol_ops.rs` — Symbol list/search/implementation/callers (AST-aware)/tests/variables (AST-aware)
  - `content.rs` — peek, grep (with scope-aware filtering), chunk_indices
  - `annotations.rs` — Save/load annotations to/from JSON on disk
  - `history.rs` — Session history retrieval
- **`config.rs`** — Hardcoded ignore patterns and max file size constant

### Data flow

1. `POST /sessions` with `{"cwd": "..."}` → `AppState::get_or_create_project()` scans the directory synchronously, then spawns background symbol extraction via `parser::extract_all_symbols()`
2. All subsequent requests include `X-Session-Id` header → `require_project()` resolves session to project
3. Filesystem watcher detects changes → re-scans affected files and re-extracts symbols
4. Annotations (definitions, marks) are in-memory with optional persistence — `POST /annotations/save` writes to `.coderlm/annotations.json` in the project root, and annotations are auto-loaded on session creation

### Concurrency model

All shared state uses `DashMap` (lock-free concurrent hashmap). The `Project.last_active` timestamp uses `parking_lot::Mutex`. Multiple sessions can read/annotate the same project concurrently. Symbol extraction runs on `tokio::spawn`, grep runs on `tokio::task::spawn_blocking`.

## API

All endpoints under `/api/v1/`. Session-scoped endpoints require `X-Session-Id` header. Admin endpoints (`GET /sessions`, `GET /history` without session, `GET /roots`) work without a session. See `server/REPL_to_API.md` for the full endpoint reference with curl examples.

## Skill CLI

The skill wraps the API with a Python CLI (no external dependencies):

```bash
python3 plugin/skills/coderlm/scripts/coderlm_cli.py <command> [args]
```

The `.claude/skills/coderlm/scripts/coderlm_cli.py` path also works when running locally (via the workaround copy).

Session state cached in `.claude/coderlm_state/session.json`. The CLI must be run from the project root that was indexed. Key commands: `init`, `structure`, `symbols`, `search`, `impl`, `callers`, `tests`, `grep` (with `--scope code`), `peek`, `save-annotations`, `load-annotations`, `cleanup`. Full reference in `plugin/skills/coderlm/references/api-reference.md`.

## Workflow: Codebase Exploration

Always use `/coderlm` (the coderlm skill) when exploring this codebase. It provides indexed lookups for symbols, implementations, callers, and tests — much faster and more precise than globbing/grepping/reading files manually. Start with `init`, then use `search`, `impl`, `callers`, `grep`, etc. See `plugin/skills/coderlm/` for full reference.
