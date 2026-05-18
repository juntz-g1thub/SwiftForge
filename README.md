# FASTCODE

## Active Project: Rust Programming Agent Platform (TUI Agent Platform)

**Context:** Rust-based TUI agent platform with streaming LLM support, tool calling, and debug panel.

---

## Current Status

**Branch:** `feature/rust-agent-phase2` (worktree: `.worktrees/rust-agent-phase2/`)
**Build:** ✅ Compiles successfully, no compile errors
**Last Updated:** 2026-05-18

---

## Architecture

```
rust-agent-platform/src/
├── main.rs                    # Binary entry point
├── lib.rs                     # Library exports
├── core/                      # Core types (Agent, Tool, Session, Provider)
├── tools/                     # Built-in tools (bash, read, write, edit, grep)
├── tui/                       # Terminal UI (ratatui-based)
│   ├── app.rs                 # Main TUI application (1102 lines)
│   ├── config.rs              # Configuration management
│   ├── components.rs          # UI components
│   └── input.rs               # Input handling
├── providers/                 # LLM providers
│   ├── openai.rs              # OpenAI GPT models
│   ├── anthropic.rs           # Anthropic Claude models
│   ├── deepseek.rs            # DeepSeek V4 (with streaming + thinking)
│   ├── ollama.rs              # Ollama local models
│   ├── minimax.rs             # MiniMax models
│   └── custom.rs              # Custom/provider
├── platform/                  # Platform features
│   ├── boulder.rs             # TODO persistence (SQLite)
│   ├── boulder_db.rs          # Boulder database
│   ├── hooks/                 # Hook system (52 hooks)
│   ├── skill/                 # Skill loading (SKILL.md)
│   ├── intent_gate.rs         # Intent classification
│   └── category.rs            # Category definitions
├── orchestration/             # Multi-agent orchestration
│   ├── agent.rs               # Agent orchestration
│   ├── scheduler.rs           # Task scheduler
│   └── message_bus.rs         # Message bus
└── integration/              # External integrations
    └── mcp/                   # MCP client (protocol, client)
```

---

## Accomplished ✅

| Feature | Status | Notes |
|---------|--------|-------|
| Project Scaffolding | ✅ | Full module structure |
| Provider Abstraction | ✅ | 6 providers with stream_chat |
| Tool System | ✅ | bash, read, write, edit, grep |
| TUI Framework | ✅ | ratatui 0.26, 1102 lines |
| Streaming Output | ✅ | stream_ui channel |
| Debug Panel | ✅ | debug_tx channel, ↑↓ scroll |
| --debug Flag | ✅ | Debug window toggle |
| Log Files | ✅ | Timestamped filenames |
| Tag Format | ✅ | `<thinking>`, `<content>`, `<tool>` |
| Boulder Persistence | ✅ | SQLite storage |
| Hook System | ✅ | 52 hooks implemented |
| Skill Loading | ✅ | SKILL.md format |
| Agent Orchestration | ✅ | Scheduler + MessageBus |
| MCP Client | ✅ | Protocol + client框架 |
| Intent Gate | ✅ | Category classification |
| Eprintln Removal | ✅ | All replaced with `let _ =` |

---

## DeepSeek V4 Streaming

**Model:** `deepseek-v4-pro`

### Key Behaviors Discovered

1. **Thinking Content**: Returns in `delta.reasoning_content` field
2. **Tool Calls**: Returns as text tags `<tool>bash</tool>` (not structured `tool_calls`)
3. **Streaming**: Requires `thinking: { type: "enabled" }` and `reasoning_effort: "high"`
4. **Tool Call Feedback**: Must send `reasoning_content` back with tool results

### Streaming Architecture

```rust
// DeepSeekProvider::stream_chat
- Extracts reasoning_content → wraps with [thinking] prefix
- Extracts content delta → outputs directly
- Channels: stream_ui for content, debug_tx for debug panel
```

---

## TUI Features

- **Layout**: Vertical split with messages area, input area, status bar
- **Debug Panel**: Toggle with `--debug` flag, ↑↓ to scroll, auto-scroll during streaming
- **Table Rendering**: Unicode box-drawing with auto-scaling columns
- **CJK Support**: Fullwidth character handling (2-cell width)
- **Input**: Command input with Enter to send

---

## Remaining Tasks

| Task | Priority | Status |
|------|----------|--------|
| DeepSeek V4 tool_calls text format parsing | HIGH | ❌ Pending |
| Accumulate reasoning_content for tool feedback | HIGH | ❌ Pending |
| Full end-to-end streaming verification | HIGH | ❌ Pending |
| Tool call execution in agent loop | HIGH | ❌ Pending |
| MCP server real HTTP communication | MEDIUM | 🔄 Partial |

---

## Build & Run

```bash
# Build
cd rust-agent-platform
cargo build --bin ragent

# Run without debug
cargo run --bin ragent

# Run with debug panel
cargo run --bin ragent -- --debug

# Tests
cargo test
```

---

## Git Workflow

```bash
# Current worktree
git worktree list
# .worktrees/rust-agent-phase2/rust-agent-platform  feature/rust-agent-phase2
# .worktrees/rust-agent-platform/rust-agent-platform  feature/rust-agent-platform

# Commit in current worktree
git add -A && git commit -m "your message"
git push -u origin feature/rust-agent-phase2
```

---

## Tech Stack

- **Agent Core:** Custom Rust (async_trait, tokio)
- **Provider:** Multi-model (OpenAI, Anthropic, DeepSeek, Ollama, MiniMax, Custom)
- **TUI:** ratatui 0.26, crossterm 0.27
- **Storage:** SQLite (rusqlite)
- **HTTP:** reqwest 0.12
- **Logging:** tracing + tracing-subscriber

---

## Phase History

| Phase | Date | Status |
|-------|------|--------|
| Phase 1: Core Infrastructure | 2026-05-04 | ✅ Complete |
| Phase 2: Platform Layer | 2026-05-11 | ✅ Complete |
| Phase 3: TUI + Streaming + Tool Calling | 2026-05-18 | 🔄 In Progress |