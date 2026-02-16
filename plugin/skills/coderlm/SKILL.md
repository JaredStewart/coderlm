---
name: coderlm
description: "Primary tool for all code navigation and reading in supported languages (Rust, Python, TypeScript, JavaScript, Go, Java, Scala, SQL). Use instead of Read, Grep, and Glob for finding symbols, reading function implementations, tracing callers, discovering tests, and understanding execution paths. Provides tree-sitter-backed indexing that returns exact source code — full function bodies, call sites with line numbers, test locations — without loading entire files into context. Use for: finding functions by name or pattern, reading specific implementations, answering 'what calls X', 'where does this error come from', 'how does X work', tracing from entrypoint to outcome, and any codebase exploration. Use Read only for config files, markdown, and unsupported languages."
allowed-tools:
  - Bash
  - Read
---

# CodeRLM — Structural Codebase Exploration

You have access to a tree-sitter-backed index server that knows the structure of this codebase: every function, every caller, every symbol, every test reference. Use it instead of guessing with grep.

The tree-sitter is monitoring the directory and will stay up-to-date as you make changes in the codebase.

## How to Explore

Do not scan files looking for relevant code. Work the way an engineer traces through a codebase:

**Start from an entrypoint.** Every exploration begins somewhere concrete — an error message, a function name, an API endpoint, a log line. Use `search` or `grep` to locate that entrypoint in the index.

**Trace the path.** Once you've found an entrypoint, use `callers` to understand what invokes it and `impl` to read what it does. Follow the chain: what calls this? What does that caller do? What state does it pass in? Build a model of the execution path, not a list of files.

**Understand the sequence of events.** The goal is to reconstruct the causal chain — what had to happen to produce the state you're looking at. Trace upstream (what called this, with what arguments?) and sometimes downstream (what happens after, does it matter?).

**Stop when you have the narrative.** You're done exploring when you can explain the path from trigger to outcome — not when you've read every related file.

## What This Replaces

Without the index, you explore by globbing for filenames, grepping for strings, and reading entire files hoping to find relevant sections. That works, but it's wasteful and produces false confidence — you see code near your search term but miss the actual execution path.

With the index:
- **Symbol search** instead of string matching — find the function, not every comment mentioning it
- **Caller chains** instead of grep-and-hope — know exactly what invokes a function
- **Exact implementations** instead of full-file reads — get the 20-line function body, not the 500-line file
- **Test discovery** by symbol reference — find what tests cover a function, not by guessing test filenames

## Prerequisites

The `coderlm-server` must be running. Start it separately:

```bash
coderlm-server serve                     # indexes projects on-demand
coderlm-server serve /path/to/project    # pre-index a specific project
```

If the server is not running, all CLI commands will fail with a connection error.

## CLI Reference

All commands go through the wrapper script:

```bash
python3 skills/coderlm/scripts/coderlm_cli.py <command> [args]
```

### Setup

```bash
cli init                                  # Create session, index the project
cli structure --depth 2                   # File tree with language breakdown
```

### Finding Code

```bash
cli search "symbol_name" --limit 20       # Find symbols by name (index lookup)
cli symbols --kind function --file path   # List all functions in a file
cli grep "pattern" --max-matches 20       # Scope-aware pattern search
```

### Retrieving Exact Code

```bash
cli impl function_name --file path        # Full function body (tree-sitter extracted)
cli peek path --start N --end M           # Exact line range
cli variables function_name --file path   # Local variables inside a function
```

**Prefer `impl` and `peek` over the Read tool.** They return exactly the code you need — a single function from a 1000-line file, a specific line range — without loading irrelevant code into context. Fall back to Read only when you need an entire small file.

### Tracing Connections

```bash
cli callers function_name --file path     # Every call site: file, line, calling code
cli tests function_name --file path       # Tests referencing this symbol
```

These search the entire indexed codebase, not just files you've already seen.

### Annotating

```bash
cli define-file src/server/mod.rs "HTTP routing and handler dispatch"
cli define-symbol handle_request --file src/server/mod.rs "Routes requests by method+path"
cli mark tests/integration.rs test
```

Annotations persist across queries within a session — build shared understanding as you go.

### Cleanup

```bash
cli cleanup                               # End session
```

## Inputs

This skill reads `$ARGUMENTS`. Accepted patterns:
- `query=<question>` (required): what to find or understand
- `cwd=<path>` (optional): project directory, defaults to cwd
- `port=<N>` (optional): server port, defaults to 3000

If no query is provided, ask what the user wants to find or understand about the codebase.

## Workflow

1. **Init** — `cli init` to create a session and index the project.
2. **Orient** — `cli structure` to see the project layout. Identify likely starting points.
3. **Find the entrypoint** — `cli search` or `cli grep` to locate the starting symbol or pattern.
4. **Retrieve** — `cli impl` to read the exact implementation. Not the file. The function.
5. **Trace** — `cli callers` to see what calls it. `cli impl` on those callers. Follow the chain.
6. **Widen** — `cli tests` to find test coverage. `cli grep` for related patterns discovered during tracing.
7. **Annotate** — `cli define-symbol` and `cli define-file` as understanding solidifies.
8. **Synthesize** — Compile findings into a coherent answer with specific file:line references.

Steps 3–7 repeat. A typical exploration is: find a symbol → read its implementation → trace its callers → read those implementations → discover related symbols → repeat until the causal chain is clear.

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
| Read an entire small file | Read tool | When you genuinely need the whole file |

**Default to the server.** Use Read only when you need an entire file or the server is unavailable.

## Troubleshooting

- **"Cannot connect to coderlm-server"** — Server not running. Start with `coderlm-server serve`.
- **"No active session"** — Run `cli init` first.
- **"Project was evicted"** — Server hit capacity (default 5 projects). Re-run `cli init`.
- **Search returns nothing relevant** — Try broader grep patterns or list all symbols: `cli symbols --limit 200`.

For the full API endpoint reference, see [references/api-reference.md](references/api-reference.md).