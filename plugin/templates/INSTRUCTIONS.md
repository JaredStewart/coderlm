# CodeRLM — Structural Codebase Exploration ({{PLATFORM_NAME}})

You have access to a tree-sitter-backed index server that knows the structure of this codebase: every function, every caller, every symbol, every test reference. Use it instead of guessing with grep.

The server monitors the directory via filesystem watcher and stays up-to-date as you make changes.

## Prerequisites

The `coderlm-server` must be running. Start it separately:

```bash
coderlm-server serve                     # indexes projects on-demand
coderlm-server serve /path/to/project    # pre-index a specific project
```

If the server is not running, all CLI commands will fail with a connection error.

## How to Explore

Do not scan files looking for relevant code. Work the way an engineer traces through a codebase:

**Start from an entrypoint.** Every exploration begins somewhere concrete — an error message, a function name, an API endpoint, a log line. Use `search` or `grep` to locate that entrypoint in the index.

**Trace the path.** Once you've found an entrypoint, use `callers` to understand what invokes it and `impl` to read what it does. Follow the chain: what calls this? What does that caller do? What state does it pass in? Build a model of the execution path, not a list of files.

**Understand the sequence of events.** The goal is to reconstruct the causal chain — what had to happen to produce the state you're looking at. Trace upstream (what called this, with what arguments?) and sometimes downstream (what happens after, does it matter?).

**Stop when you have the narrative.** You're done exploring when you can explain the path from trigger to outcome — not when you've read every related file.

## CLI Reference

All commands go through the wrapper script:

```bash
python3 {{CLI_PATH}} <command> [args]
```

### Setup

```bash
python3 {{CLI_PATH}} init                    # Create session, index the project
python3 {{CLI_PATH}} structure --depth 2     # File tree with language breakdown
```

### Finding Code

```bash
python3 {{CLI_PATH}} search "symbol_name" --limit 20     # Find symbols by name (index lookup)
python3 {{CLI_PATH}} symbols --kind function --file path  # List all functions in a file
python3 {{CLI_PATH}} grep "pattern" --max-matches 20      # Scope-aware pattern search
python3 {{CLI_PATH}} grep "pattern" --scope code           # Skip matches in comments/strings
```

### Retrieving Exact Code

```bash
python3 {{CLI_PATH}} impl function_name --file path        # Full function body (tree-sitter extracted)
python3 {{CLI_PATH}} peek path --start N --end M           # Exact line range
python3 {{CLI_PATH}} variables function_name --file path   # Local variables inside a function
```

**Prefer `impl` and `peek` over reading entire files.** They return exactly the code you need — a single function from a 1000-line file, a specific line range — without loading irrelevant code into context.

### Tracing Connections

```bash
python3 {{CLI_PATH}} callers function_name --file path     # Every call site: file, line, calling code
python3 {{CLI_PATH}} tests function_name --file path       # Tests referencing this symbol
```

These search the entire indexed codebase, not just files you've already seen.

### Annotating

```bash
python3 {{CLI_PATH}} define-file src/server/mod.rs "HTTP routing and handler dispatch"
python3 {{CLI_PATH}} define-symbol handle_request --file src/server/mod.rs "Routes requests by method+path"
python3 {{CLI_PATH}} mark tests/integration.rs test
python3 {{CLI_PATH}} save-annotations                      # Persist to disk
python3 {{CLI_PATH}} load-annotations                      # Reload from disk
```

Annotations persist across queries within a session. Use `save-annotations` to persist across sessions.

### Cleanup

```bash
python3 {{CLI_PATH}} cleanup                               # End session
```

## Workflow

1. **Init** — `init` to create a session and index the project.
2. **Orient** — `structure` to see the project layout. Identify likely starting points.
3. **Find the entrypoint** — `search` or `grep` to locate the starting symbol or pattern.
4. **Retrieve** — `impl` to read the exact implementation. Not the file. The function.
5. **Trace** — `callers` to see what calls it. `impl` on those callers. Follow the chain.
6. **Widen** — `tests` to find test coverage. `grep` for related patterns discovered during tracing.
7. **Annotate** — `define-symbol` and `define-file` as understanding solidifies.
8. **Synthesize** — Compile findings into a coherent answer with specific file:line references.

Steps 3-7 repeat. A typical exploration is: find a symbol -> read its implementation -> trace its callers -> read those implementations -> discover related symbols -> repeat until the causal chain is clear.

## When to Use the Server vs Native Tools

| Task | Use server | Why |
|------|-----------|-----|
| Find a function by name | `search` | Index lookup, not file globbing |
| Find code when name is unknown | `grep` + `symbols` | Searches all indexed files at once |
| Get a function's source | `impl` | Returns just that function, even from large files |
| Read specific lines | `peek` | Surgical extraction, not the whole file |
| Find what calls a function | `callers` | Cross-project search with exact call sites |
| Find tests for a function | `tests` | By symbol reference, not filename guessing |
| Get project overview | `structure` | Tree with file counts and language breakdown |
| Read an entire small file | Native read | When you genuinely need the whole file |

**Default to the server.** Use native file reading only when you need an entire file or the server is unavailable.

## Troubleshooting

- **"Cannot connect to coderlm-server"** — Server not running. Start with `coderlm-server serve`.
- **"No active session"** — Run `init` first.
- **"Project was evicted"** — Server hit capacity (default 5 projects). Re-run `init`.
- **Search returns nothing relevant** — Try broader grep patterns or list all symbols: `symbols --limit 200`.

## Supported Languages

| Language   | Extensions                    |
|------------|-------------------------------|
| Rust       | `.rs`                         |
| Python     | `.py`, `.pyi`                 |
| TypeScript | `.ts`, `.tsx`                 |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` |
| Go         | `.go`                         |

All file types appear in the file tree and are searchable via peek/grep, but only the above produce parsed symbols.
