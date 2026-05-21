# FASTCODE

## Project

Rust-based TUI agent platform with streaming LLM support, tool calling, and debug panel.

---

## Architecture

```
rust-agent-platform/src/
├── main.rs                    # Binary entry point
├── lib.rs                     # Library exports
├── core/                      # Core types (Agent, Tool, Session, Provider)
├── tools/                     # Built-in tools (bash, read, write, edit, grep)
├── tui/                       # Terminal UI (ratatui-based)
│   ├── app_controller.rs      # Main controller
│   ├── components/            # UI components (input_area, message_list, scroll_bar, status_bar)
│   ├── state/                 # State management (action, app_context, view_state)
│   └── views/                 # Views (chat_view, config_view, debug_view)
├── providers/                 # LLM providers
│   ├── openai.rs              # OpenAI GPT models
│   ├── anthropic.rs           # Anthropic Claude models
│   ├── deepseek.rs            # DeepSeek models
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
│   ├── scheduler.rs           # Task scheduler
│   └── message_bus.rs         # Message bus
└── integration/              # External integrations
    └── mcp/                   # MCP client (protocol, client)
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
52 hooks defined in `platform/hooks/`. Hook lifecycle events for agent orchestration.
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
cd rust-agent-platform

# Build
cargo build --bin ragent

# Run without debug
cargo run --bin ragent

# Run with debug panel
cargo run --bin ragent -- --debug

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