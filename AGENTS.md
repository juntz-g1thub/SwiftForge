# AGENTS.md

## Project Overview

**rust-agent-platform** — Rust TUI agent platform with streaming LLM support and tool calling.
**Language**: Rust
**Source**: `rust-agent-platform/` (workspace member)

## Git Workflow

详细规范见 [.git-workflow.md](.git-workflow.md)。

**分支命名**: `类型/范围-名称`（feat, fix, refactor, chore, docs, test, perf）
**工作树**: `.worktrees/{branch-name}`（`/` 替换为 `-`）
**工作树列表**: `git worktree list`

> ⚠️ 所有 git 操作必须由用户手动执行，切勿自动运行 git 命令。

## Architecture

```
rust-agent-platform/src/
├── core/             # Agent, Tool, Session, Provider (trait)
├── providers/         # 6 LLM providers: openai, anthropic, deepseek, ollama, minimax, custom
├── tools/            # Built-in tools: bash, read, write, edit, grep
├── tui/              # ratatui-based terminal UI
├── platform/         # Boulder (SQLite TODO), Hooks (52), Skills (SKILL.md), Intent Gate
├── orchestration/    # TaskScheduler, MessageBus, multi-agent
└── integration/mcp/  # MCP client
```

## Key Conventions

### Intent Gate (routing)
`IntentCategory` enum: Research, Implementation, Investigation, Evaluation, Fix, OpenEnded, Trivial.
Routes to: `explore/librarian → synthesize`, `plan → delegate`, `explore → report`, etc.

### Skill Loading
Skills defined in `SKILL.md` files with YAML frontmatter (`name`, `description`, `scope: Global|Project|User`).
Loaded via `SkillLoader::load_skill(path)`.

### Hook System
52 hooks defined in `platform/hooks/`. Hook lifecycle events for agent orchestration.
Use `HookRegistry` to register callbacks.

### Boulder (TODO)
SQLite-backed TODO persistence via `BoulderStore`. Uses temp directories for test isolation.

## Dev Notes

- **No CI/CD**: No GitHub Actions, no pre-commit hooks found
- **No custom linting**: Uses default `cargo clippy` / `cargo fmt`
- **Logging**: `tracing` + `tracing-subscriber` → stderr (no log files unless `--debug`)
- **Session files**: `session*.md` in root are ignored by `.gitignore`