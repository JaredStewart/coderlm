#!/usr/bin/env python3
"""Generate CodeRLM instruction files for AI coding platforms.

Renders a universal instruction template with platform-specific paths and copies
the CLI script to the expected location. No external dependencies required.

Usage:
    python3 plugin/generate.py --platform cursor       # Single platform
    python3 plugin/generate.py --platform all           # All platforms
    python3 plugin/generate.py --list                   # List platforms
    python3 plugin/generate.py --platform cursor --dry-run
    python3 plugin/generate.py --platform cursor --clean
"""

from __future__ import annotations

import argparse
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

SCRIPT_DIR = Path(__file__).resolve().parent
TEMPLATE_PATH = SCRIPT_DIR / "templates" / "INSTRUCTIONS.md"
CLI_SOURCE = SCRIPT_DIR / "skills" / "coderlm" / "scripts" / "coderlm_cli.py"

CODEX_MARKER_START = "<!-- coderlm-start -->"
CODEX_MARKER_END = "<!-- coderlm-end -->"


@dataclass
class Platform:
    name: str
    instruction_path: str  # relative to project root
    cli_path: str  # relative to project root (where CLI gets copied)
    state_dir: str  # state directory for this platform
    format: str  # "markdown", "mdc", "append"
    mdc_frontmatter: Optional[str] = None  # for Cursor's .mdc format


PLATFORMS: dict[str, Platform] = {
    "cursor": Platform(
        name="Cursor",
        instruction_path=".cursor/rules/coderlm.mdc",
        cli_path=".cursor/coderlm/coderlm_cli.py",
        state_dir=".cursor/coderlm/state",
        format="mdc",
        mdc_frontmatter=(
            "---\n"
            "description: CodeRLM - tree-sitter-backed codebase exploration via index server\n"
            "globs:\n"
            "alwaysApply: true\n"
            "---\n"
        ),
    ),
    "windsurf": Platform(
        name="Windsurf",
        instruction_path=".windsurf/rules/coderlm.md",
        cli_path=".windsurf/coderlm/coderlm_cli.py",
        state_dir=".windsurf/coderlm/state",
        format="markdown",
    ),
    "copilot": Platform(
        name="GitHub Copilot",
        instruction_path=".github/instructions/coderlm.instructions.md",
        cli_path=".github/coderlm/coderlm_cli.py",
        state_dir=".github/coderlm/state",
        format="markdown",
    ),
    "roo": Platform(
        name="Roo Code",
        instruction_path=".roo/rules/coderlm.md",
        cli_path=".roo/coderlm/coderlm_cli.py",
        state_dir=".roo/coderlm/state",
        format="markdown",
    ),
    "kilo": Platform(
        name="Kilo Code",
        instruction_path=".kilo/rules/coderlm.md",
        cli_path=".kilo/coderlm/coderlm_cli.py",
        state_dir=".kilo/coderlm/state",
        format="markdown",
    ),
    "gemini": Platform(
        name="Gemini CLI",
        instruction_path=".gemini/instructions/coderlm.md",
        cli_path=".gemini/coderlm/coderlm_cli.py",
        state_dir=".gemini/coderlm/state",
        format="markdown",
    ),
    "codex": Platform(
        name="Codex CLI",
        instruction_path="AGENTS.md",
        cli_path=".codex/coderlm/coderlm_cli.py",
        state_dir=".codex/coderlm/state",
        format="append",
    ),
    "opencode": Platform(
        name="OpenCode",
        instruction_path=".opencode/instructions/coderlm.md",
        cli_path=".opencode/coderlm/coderlm_cli.py",
        state_dir=".opencode/coderlm/state",
        format="markdown",
    ),
    "augment": Platform(
        name="Augment Code",
        instruction_path=".augment/instructions/coderlm.md",
        cli_path=".augment/coderlm/coderlm_cli.py",
        state_dir=".augment/coderlm/state",
        format="markdown",
    ),
    "amazonq": Platform(
        name="Amazon Q",
        instruction_path=".amazonq/rules/coderlm.md",
        cli_path=".amazonq/coderlm/coderlm_cli.py",
        state_dir=".amazonq/coderlm/state",
        format="markdown",
    ),
    "amp": Platform(
        name="Amp",
        instruction_path=".amp/rules/coderlm.md",
        cli_path=".amp/coderlm/coderlm_cli.py",
        state_dir=".amp/coderlm/state",
        format="markdown",
    ),
    "qwen": Platform(
        name="Qwen Code",
        instruction_path=".qwen/rules/coderlm.md",
        cli_path=".qwen/coderlm/coderlm_cli.py",
        state_dir=".qwen/coderlm/state",
        format="markdown",
    ),
}


def load_template() -> str:
    if not TEMPLATE_PATH.exists():
        print(f"ERROR: Template not found: {TEMPLATE_PATH}", file=sys.stderr)
        sys.exit(1)
    return TEMPLATE_PATH.read_text()


def render_template(template: str, platform: Platform) -> str:
    content = template.replace("{{CLI_PATH}}", platform.cli_path)
    content = content.replace("{{STATE_DIR}}", platform.state_dir)
    content = content.replace("{{PLATFORM_NAME}}", platform.name)
    return content


def generate_platform(platform: Platform, project_root: Path, dry_run: bool) -> None:
    template = load_template()
    rendered = render_template(template, platform)

    instruction_file = project_root / platform.instruction_path
    cli_dest = project_root / platform.cli_path

    # Instruction file
    if platform.format == "mdc":
        content = (platform.mdc_frontmatter or "") + "\n" + rendered
        _write_file(instruction_file, content, dry_run)

    elif platform.format == "append":
        _append_with_markers(instruction_file, rendered, dry_run)

    else:  # markdown
        _write_file(instruction_file, rendered, dry_run)

    # Copy CLI script
    _copy_file(CLI_SOURCE, cli_dest, dry_run)

    # Write env hint
    env_hint = (
        f"  Set CODERLM_STATE_DIR={platform.state_dir} when running the CLI\n"
        f"  (or the CLI defaults to .claude/coderlm_state)"
    )
    print(f"  Note: {env_hint}")


def clean_platform(platform: Platform, project_root: Path, dry_run: bool) -> None:
    instruction_file = project_root / platform.instruction_path
    cli_dest = project_root / platform.cli_path
    cli_dir = cli_dest.parent
    state_dir = project_root / platform.state_dir

    if platform.format == "append":
        _remove_markers(instruction_file, dry_run)
    else:
        _remove_file(instruction_file, dry_run)

    _remove_file(cli_dest, dry_run)

    # Remove state dir if empty
    _remove_dir_if_empty(state_dir, dry_run)
    # Remove CLI parent dir if empty
    _remove_dir_if_empty(cli_dir, dry_run)


# -- File operations --


def _write_file(path: Path, content: str, dry_run: bool) -> None:
    if dry_run:
        print(f"  [dry-run] write {path}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    print(f"  Created {path}")


def _copy_file(src: Path, dest: Path, dry_run: bool) -> None:
    if not src.exists():
        print(f"  WARNING: source not found: {src}", file=sys.stderr)
        return
    if dry_run:
        print(f"  [dry-run] copy {src} -> {dest}")
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)
    print(f"  Copied {dest}")


def _append_with_markers(path: Path, content: str, dry_run: bool) -> None:
    block = f"\n{CODEX_MARKER_START}\n{content}\n{CODEX_MARKER_END}\n"

    if path.exists():
        existing = path.read_text()
        if CODEX_MARKER_START in existing:
            if dry_run:
                print(f"  [dry-run] replace marked section in {path}")
                return
            # Replace existing block
            start = existing.index(CODEX_MARKER_START)
            end = existing.index(CODEX_MARKER_END) + len(CODEX_MARKER_END)
            updated = existing[:start] + CODEX_MARKER_START + "\n" + content + "\n" + CODEX_MARKER_END + existing[end + len(CODEX_MARKER_END):]
            # Simpler: just reconstruct
            before = existing[:start].rstrip("\n")
            after = existing[end:].lstrip("\n")
            parts = [before, block.strip(), after]
            updated = "\n\n".join(p for p in parts if p) + "\n"
            path.write_text(updated)
            print(f"  Updated marked section in {path}")
            return

    if dry_run:
        print(f"  [dry-run] append to {path}")
        return

    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a") as f:
        f.write(block)
    print(f"  Appended CodeRLM section to {path}")


def _remove_markers(path: Path, dry_run: bool) -> None:
    if not path.exists():
        print(f"  Already absent: {path}")
        return

    content = path.read_text()
    if CODEX_MARKER_START not in content:
        print(f"  No CodeRLM section in {path}")
        return

    if dry_run:
        print(f"  [dry-run] remove marked section from {path}")
        return

    start = content.index(CODEX_MARKER_START)
    end = content.index(CODEX_MARKER_END) + len(CODEX_MARKER_END)
    before = content[:start].rstrip("\n")
    after = content[end:].lstrip("\n")
    updated = before + ("\n\n" + after if after else "") + "\n" if before else after
    path.write_text(updated)
    print(f"  Removed CodeRLM section from {path}")


def _remove_file(path: Path, dry_run: bool) -> None:
    if not path.exists():
        print(f"  Already absent: {path}")
        return
    if dry_run:
        print(f"  [dry-run] remove {path}")
        return
    path.unlink()
    print(f"  Removed {path}")


def _remove_dir_if_empty(path: Path, dry_run: bool) -> None:
    if not path.exists() or not path.is_dir():
        return
    try:
        if any(path.iterdir()):
            return
    except PermissionError:
        return
    if dry_run:
        print(f"  [dry-run] rmdir {path}")
        return
    path.rmdir()
    print(f"  Removed empty directory {path}")


# -- CLI --


def list_platforms() -> None:
    print("Available platforms:\n")
    print(f"  {'Name':<16} {'Instruction File':<48} {'Format'}")
    print(f"  {'-'*15} {'-'*47} {'-'*10}")
    for key, p in PLATFORMS.items():
        print(f"  {key:<16} {p.instruction_path:<48} {p.format}")
    print(f"\nUse --platform <name> to generate, or --platform all for everything.")
    print(f"Claude Code users: use the plugin marketplace instead (no generation needed).")


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="coderlm",
        description="Generate CodeRLM instruction files for AI coding platforms",
    )
    parser.add_argument(
        "--platform",
        help="Platform to generate for (or 'all')",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List available platforms",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be done without writing files",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Remove generated files for the specified platform",
    )
    parser.add_argument(
        "--project-root",
        type=Path,
        default=Path.cwd(),
        help="Project root directory (default: current directory)",
    )

    args = parser.parse_args()

    if args.list:
        list_platforms()
        return

    if not args.platform:
        parser.print_help()
        sys.exit(1)

    # Resolve platforms
    if args.platform == "all":
        targets = list(PLATFORMS.values())
    elif args.platform in PLATFORMS:
        targets = [PLATFORMS[args.platform]]
    else:
        print(f"ERROR: Unknown platform '{args.platform}'", file=sys.stderr)
        print(f"Available: {', '.join(PLATFORMS.keys())}, all", file=sys.stderr)
        sys.exit(1)

    project_root = args.project_root.resolve()
    action = "clean" if args.clean else "generate"

    if args.dry_run:
        print(f"[DRY RUN] {action} for: {', '.join(t.name for t in targets)}")
    else:
        print(f"{action.title()} for: {', '.join(t.name for t in targets)}")
    print(f"Project root: {project_root}\n")

    for platform in targets:
        print(f"[{platform.name}]")
        if args.clean:
            clean_platform(platform, project_root, args.dry_run)
        else:
            generate_platform(platform, project_root, args.dry_run)
        print()

    if not args.dry_run and not args.clean:
        print("Done. Remember to start the coderlm-server before using the CLI.")


if __name__ == "__main__":
    main()
