---
name: coderlm
description: Explore a codebase using tree-sitter-backed indexing. Use when you need to understand how code works, trace execution paths, find where errors originate, or understand the sequence of events that produce a particular outcome. Prefer this over grep/glob/read for structural code questions.
---

# CodeRLM — Structural Codebase Exploration

You have access to a tree-sitter-backed index server that knows the structure of this codebase: every function, every caller, every symbol. Use it instead of guessing with grep.

## Setup

```bash
CLI=".claude/coderlm_state/coderlm_cli.py"
# Session is auto-created by the plugin. If not:
python3 $CLI init
```

## Deep Exploration

For tracing execution paths across multiple files, delegate to a coderlm-subcall agent:

```
Agent(coderlm-subcall, "Trace how <X> works...")
```

## Quick Lookups

```bash
# One-off command
python3 $CLI search "symbol_name"

# Batch multiple queries in one call
python3 $CLI batch --commands "
search symbol_name
impl symbol_name --file path/to/file.py
callers symbol_name --file path/to/file.py
"

# Programmatic exploration with conditional logic
python3 $CLI exec --code "
results = search('symbol_name')
if results.get('symbols'):
    sym = results['symbols'][0]
    impl_(sym['name'], sym['file'])
"
```

## All Commands

```bash
python3 $CLI structure                          # File tree + module overview
python3 $CLI search "symbol_name"               # Find symbols by name
python3 $CLI impl function_name --file path     # Get exact implementation
python3 $CLI callers function_name --file path  # Who calls this function?
python3 $CLI tests symbol --file path           # Find tests covering this symbol
python3 $CLI grep "pattern" --scope code        # Scope-aware pattern search
python3 $CLI peek path --start N --end N        # Read a specific line range
python3 $CLI symbols --file path --kind KIND    # List symbols in a file
python3 $CLI variables func --file path         # List local variables
```

## How to Explore

Do not scan files looking for relevant code. Instead, work the way a human engineer traces through a codebase:

**Start from an entrypoint.** Every exploration begins somewhere concrete — an error message, a function name, an API endpoint, a log line. Use `grep` or `search` to locate that entrypoint in the index.

**Trace the path.** Once you've found the entrypoint, use `callers` to understand what invokes it and `impl` to read what it does. Follow the chain: what calls this? What does that caller do? What state does it pass in?

**Stop when you have the narrative.** You're done exploring when you can explain the path from trigger to outcome — not when you've read every related file.

## $ARGUMENTS
