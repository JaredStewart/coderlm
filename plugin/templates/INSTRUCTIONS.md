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
python3 {{CLI_PATH}} init                    # Create session, index the project (response includes L1 structure for free)
python3 {{CLI_PATH}} structure --depth 2     # File tree with language breakdown
```

### Finding Code

```bash
python3 {{CLI_PATH}} search "symbol_name" --limit 20             # Find symbols by name (index lookup)
python3 {{CLI_PATH}} search "symbol_name" --file path/to/file.py  # Restrict search to one file
python3 {{CLI_PATH}} symbols --kind function --file path          # List all functions in a file
python3 {{CLI_PATH}} grep "pattern" --max-matches 20              # Regex search
python3 {{CLI_PATH}} grep "pattern" --scope code                   # Skip matches in comments/strings
python3 {{CLI_PATH}} grep "pattern" --file path/to/file.py        # Restrict grep to one file
```

### Retrieving Exact Code

```bash
python3 {{CLI_PATH}} impl function_name --file path        # Full function body (tree-sitter extracted)
python3 {{CLI_PATH}} peek path --start N --end M           # Exact line range
python3 {{CLI_PATH}} variables function_name --file path   # Local variables inside a function
python3 {{CLI_PATH}} chunks path --size 5000 --overlap 200 # Byte-range chunk boundaries (large files)
```

**Prefer `impl` and `peek` over reading entire files.** They return exactly the code you need — a single function from a 1000-line file, a specific line range — without loading irrelevant code into context.

### Tracing Connections

```bash
python3 {{CLI_PATH}} callers function_name --file path     # Every call site: file, line, calling code
python3 {{CLI_PATH}} tests function_name --file path       # Tests referencing this symbol
```

These search the entire indexed codebase, not just files you've already seen.

### Batch and Exec Modes (Minimize Tool Calls)

For 2+ related lookups, use `batch` (sequential CLI commands) or `exec` (Python code with helpers in scope) instead of issuing individual calls:

```bash
# Batch — runs each line as a CLI subcommand, in order, sharing the session
python3 {{CLI_PATH}} batch --commands "
  search MyFunction
  impl MyFunction --file path/to/file.py
  callers MyFunction --file path/to/file.py
"

# Exec — Python with helpers in scope; later queries can use earlier results
python3 {{CLI_PATH}} exec --code "
  hits = search('serialize_response')
  if hits.get('symbols'):
      sym = hits['symbols'][0]
      impl_(sym['name'], sym['file'])           # read implementation
      c = callers(sym['name'], sym['file'])     # find callers
      if c.get('callers'):
          caller = c['callers'][0]
          impl_(caller['name'], caller['file']) # walk one hop up
"
```

`exec` helpers in scope: `search()`, `impl_()`, `callers()`, `tests()`, `grep()`, `symbols()`, `peek_file()`, `structure()`, `variables_list()`. Use `batch` for known lookups; use `exec` when later queries depend on earlier results, or when tracing 3+ hops.

### Annotating

```bash
python3 {{CLI_PATH}} define-file src/server/mod.rs "HTTP routing and handler dispatch"
python3 {{CLI_PATH}} define-symbol handle_request --file src/server/mod.rs "Routes requests by method+path"
python3 {{CLI_PATH}} mark tests/integration.rs test
python3 {{CLI_PATH}} save-annotations                      # Persist to disk (.coderlm/annotations.json)
python3 {{CLI_PATH}} load-annotations                      # Reload from disk (also auto-loaded on init)
```

Annotations persist across queries within a session. Use `save-annotations` to persist across sessions.

### Cleanup

```bash
python3 {{CLI_PATH}} cleanup                               # End session
```

## Workflow

1. **Init** — `init` to create a session and index the project. The response already includes the L1 file/symbol structure.
2. **Orient** — `structure` if you need more depth. Identify likely starting points.
3. **Find the entrypoint** — `search` or `grep` to locate the starting symbol or pattern.
4. **Retrieve** — `impl` to read the exact implementation. Not the file. The function.
5. **Trace** — `callers` to see what calls it. `impl` on those callers. Follow the chain.
6. **Widen** — `tests` to find test coverage. `grep` for related patterns discovered during tracing.
7. **Annotate** — `define-symbol` and `define-file` as understanding solidifies; `save-annotations` to persist.
8. **Synthesize** — Compile findings into a coherent answer with specific file:line references.

Steps 3-7 repeat. A typical exploration is: find a symbol → read its implementation → trace its callers → read those implementations → discover related symbols → repeat until the causal chain is clear. Whenever the next 2+ queries are predictable, collapse them into a single `batch` or `exec` call.

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
| Multi-step trace (3+ hops) | `exec` | One call returns the whole chain |
| 2–5 known lookups in a row | `batch` | One call instead of N |
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
| Java       | `.java`                       |
| Scala      | `.scala`, `.sc`               |
| Ruby       | `.rb`, `.rake`                |
| PHP        | `.php`, `.phtml`              |
| Zig        | `.zig`, `.zon`                |

All file types appear in the file tree and are searchable via peek/grep, but only the above produce parsed symbols.
