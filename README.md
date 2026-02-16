# CodeRLM

CodeRLM applies the [Recursive Language Model](https://arxiv.org/abs/2512.24601) (RLM) pattern to codebases. A Rust server indexes a project's files and symbols via tree-sitter, then exposes a JSON API that LLM agents query for targeted context — structure, symbols, source code, callers, tests, and grep. Instead of loading an entire codebase into context or relying on heuristic file scanning, the agent asks the server for exactly what it needs.

An integrated Claude Code skill (`plugin/skills/coderlm/`) wraps the API with a Python CLI and a structured workflow, so Claude Code can explore unfamiliar codebases without reading everything into context.

## How It Works

The RLM pattern treats a codebase as external data that a root language model can recursively examine and decompose:

1. **Index** — The server walks the project directory (respecting `.gitignore`), parses every supported file with tree-sitter, and builds a symbol table with cross-references.
2. **Query** — The agent queries the index: search symbols by name, list functions in a file, find callers of a function, grep for patterns, retrieve exact source code.
3. **Read** — The server returns the exact code requested — full function implementations, variable lists, line ranges — so the agent never guesses.

This replaces the typical glob/grep/read cycle with precise, index-backed lookups.

## Origins

This project builds on two prior works:

- **"Recursive Language Models"** by Alex L. Zhang, Tim Kraska, and Omar Khattab (MIT CSAIL, 2025). The paper introduces the RLM framework for processing inputs far beyond model context windows by treating extended prompts as external data that the model recursively examines.
  > Zhang, A. L., Kraska, T., & Khattab, O. (2025). Recursive Language Models. *arXiv preprint* [arXiv:2512.24601](https://arxiv.org/abs/2512.24601).

- **[brainqub3/claude_code_RLM](https://github.com/brainqub3/claude_code_RLM)** — A minimal RLM implementation for Claude Code by brainqub3 that applies the pattern to document processing via a persistent Python REPL. CodeRLM adapts this approach from documents to codebases, replacing the Python REPL with a purpose-built Rust server and tree-sitter indexing.

## Repository Layout

```
server/                          Rust server (the only built artifact)
plugin/                          Self-contained Claude Code plugin
  plugin/skills/coderlm/         Skill definition + Python CLI wrapper
  plugin/hooks/                  Claude Code hooks (SessionStart, UserPromptSubmit, PreCompact, Stop)
  plugin/commands/               Slash command definitions
  plugin/scripts/                Hook scripts (session lifecycle)
  plugin/.claude-plugin/         Plugin manifest (plugin.json)
.claude-plugin/                  Marketplace manifest (points to plugin/)
```

## Quick Start

### Prerequisites

- **Rust toolchain** — required to build the server (`rustup` recommended)
- **Python 3** — required for the CLI wrapper (stdlib only, no pip packages)

### 1. Build and Start the Server

```bash
git clone https://github.com/JaredStewart/coderlm.git
cd coderlm/server
cargo build --release

# Start the server (in a separate terminal or as a daemon)
cargo run --release -- serve

# Or run as a daemon
./coderlm-daemon.sh start
./coderlm-daemon.sh status
./coderlm-daemon.sh stop
```

Verify:

```bash
curl http://127.0.0.1:3000/api/v1/health
# → {"status":"ok","projects":0,"active_sessions":0,"max_projects":5}
```

### 2. Install for Your AI Tool

#### Claude Code

```bash
# Add the marketplace source first, then install the plugin
claude /plugin marketplace add JaredStewart/coderlm
claude plugin install coderlm
```

After installation, the `/coderlm` skill is available in every session. The `SessionStart` hook auto-initializes and the `UserPromptSubmit` hook guides Claude to use indexed lookups.

#### Other AI Platforms (Cursor, Windsurf, Copilot, Gemini, Codex, etc.)

Install the generator:

```bash
uv tool install coderlm --from git+https://github.com/JaredStewart/coderlm.git
```

Generate for your platform:

```bash
coderlm --platform cursor
coderlm --list                    # see all supported platforms
```

Or run without installing:

```bash
uvx --from git+https://github.com/JaredStewart/coderlm.git coderlm --platform cursor
```

Or from a cloned repo:

```bash
python3 plugin/generate.py --platform cursor
```

Use `--list` to see all platforms, `--dry-run` to preview, `--clean` to remove generated files, and `--platform all` for everything.

### 3. Use the CLI

Once the server is running, invoke the skill (Claude Code) or use the CLI directly:

```bash
# Claude Code
/coderlm query="how does authentication work?"

# Direct CLI usage
python3 plugin/skills/coderlm/scripts/coderlm_cli.py init
python3 plugin/skills/coderlm/scripts/coderlm_cli.py search "handler"
python3 plugin/skills/coderlm/scripts/coderlm_cli.py impl run_server --file src/main.rs
```

### Updating

```bash
# Claude Code plugin
claude plugin update coderlm

# Other platforms — pull and regenerate
git pull
python3 plugin/generate.py --platform cursor
```

Rebuild the server after any update:

```bash
cd server && cargo build --release
```

## What the Plugin Provides

When installed, CodeRLM gives Claude Code:

- **`/coderlm` skill** — Structured workflow for codebase exploration (init → structure → search → impl → callers → synthesize)
- **SessionStart hook** — Auto-detects a running server and initializes sessions
- **UserPromptSubmit hook** — Guides Claude to use indexed lookups instead of glob/grep/read
- **Zero Python dependencies** — The CLI wrapper uses only the Python standard library

## Server CLI

```
coderlm-server serve [PATH] [OPTIONS]

Options:
  -p, --port <PORT>              Port to listen on [default: 3000]
  -b, --bind <ADDR>              Bind address [default: 127.0.0.1]
      --max-file-size <BYTES>    Max file size to index [default: 1048576]
      --max-projects <N>         Max concurrent indexed projects [default: 5]
```

## Supported Languages

| Language   | Extensions                    | Support      |
|------------|-------------------------------|--------------|
| Rust       | `.rs`                         | tree-sitter  |
| Python     | `.py`, `.pyi`                 | tree-sitter  |
| TypeScript | `.ts`, `.tsx`                 | tree-sitter  |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | tree-sitter  |
| Go         | `.go`                         | tree-sitter  |
| Java       | `.java`                       | tree-sitter  |
| Scala      | `.scala`, `.sc`               | tree-sitter  |
| SQL        | `.sql`                        | regex        |

Languages with tree-sitter support produce full symbol tables (functions, classes, methods, callers, variables). SQL uses regex fallbacks for variable and definition detection. All file types appear in the file tree and are searchable via peek/grep.

## API

All endpoints under `/api/v1/`. See [`server/REPL_to_API.md`](server/REPL_to_API.md) for the full endpoint reference with curl examples.

## License

MIT
