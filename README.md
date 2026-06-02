# SwiftForge

## Project

**SwiftForge** — 高效 Agent 运行平台，支持开发者构建和运行自己的 Agent。

Rust-based TUI agent platform with streaming LLM support, tool calling, MCP integration, and debug panel.

---

## Workspace Structure

```
SwiftForge/
├── Cargo.toml                  # Workspace root
├── swiftforge/                  # Main application crate
│   ├── src/
│   │   ├── main.rs              # Binary entry point
│   │   ├── lib.rs
│   │   ├── core/               # Agent core (agent.rs, session_manager.rs)
│   │   ├── platform/            # Boulder, intent_gate, category
│   │   └── tui/                 # Terminal UI (ratatui-based)
│   │       ├── app_controller.rs
│   │       ├── components/       # UI components
│   │       ├── state/           # State management
│   │       ├── views/           # Chat, config, debug views
│   │       └── task/            # Task coordinator & events
│   └── Cargo.toml
│
└── libs/                        # Workspace members
    ├── swiftforge-types/         # Core types (Message, Tool, Provider, Session)
    ├── swiftforge-task/          # Task scheduler & message bus
    ├── swiftforge-tools/         # Built-in tools (bash, read, write, edit, grep)
    ├── swiftforge-providers/     # LLM providers
    │   ├── openai.rs
    │   ├── anthropic.rs
    │   ├── deepseek.rs
    │   ├── ollama.rs
    │   ├── minimax.rs
    │   └── custom.rs
    ├── swiftforge-provider-core/ # Provider traits & registry
    ├── swiftforge-mcp/          # MCP client (protocol, client, pool, loader)
    ├── swiftforge-hooks/        # Hook system (52 hooks)
    ├── swiftforge-skill/         # Skill loader (SKILL.md)
    └── swiftforge-log/          # Logging (tracing-based)
```

---

## Key Conventions

### Intent Gate (routing)
`IntentCategory` enum: Research, Implementation, Investigation, Evaluation, Fix, OpenEnded, Trivial.
Routes to: `explore/librarian → synthesize`, `plan → delegate`, `explore → report`, etc.

### Skill Loading (Project Feature)
Skills defined in `SKILL.md` files with YAML frontmatter (`name`, `description`, `scope: Global|Project|User`).
Loaded via `SkillLoader::load_skill(path)`.

### Hook System
52 hooks defined in `swiftforge-hooks/`. Hook lifecycle events for agent orchestration.
Use `HookRegistry` to register callbacks.

### Boulder (TODO)
SQLite-backed TODO persistence via `BoulderStore`. Uses temp directories for test isolation.

---

## TUI Features

- **Layout**: Vertical split with messages area, input area, status bar
- **Debug Panel**: Toggle with `--debug` flag, ↑↓ to scroll, auto-scroll during streaming
- **Table Rendering**: Unicode box-drawing with auto-scaling columns
- **CJK Support**: Fullwidth character handling (2-cell width)
- **Input**: Command input with Enter to send

---

## Build & Run

```bash
# Build
cargo build --bin swiftforge

# Run without debug
cargo run --bin swiftforge

# Run with debug panel
cargo run --bin swiftforge -- --debug

# Tests
cargo test
```

---

## Tech Stack

- **Agent Core:** Custom Rust (async_trait, tokio)
- **Provider:** Multi-model (OpenAI, Anthropic, DeepSeek, Ollama, MiniMax, Custom)
- **TUI:** ratatui 0.26, crossterm 0.27
- **Storage:** SQLite (rusqlite)
- **HTTP:** reqwest 0.12
- **Logging:** tracing + tracing-subscriber