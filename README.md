# FASTCODE

## Active Project: Rust Programming Agent Platform

**Plan:** `docs/superpowers/plans/2026-05-04-rust-agent-platform-phase1.md`

### Execution Mode
- **Selected:** Subagent-Driven
- **Skill:** `superpowers:subagent-driven-development`
- **Next Action:** Fix TUI table rendering

### Project Structure
```
FastCode/
├── Cargo.toml              # Workspace
├── rust-agent-platform/    # Main crate
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── core/           # Agent, Tool, Session, Provider
│   │   ├── tools/          # bash, read, write, edit, grep
│   │   ├── tui/            # Terminal UI
│   │   ├── providers/      # OpenAI, Anthropic, Ollama, DeepSeek, MiniMax, Custom
│   │   ├── platform/       # Intent Gate, Hook, Skill
│   │   ├── orchestration/  # Multi-agent
│   │   └── integration/    # MCP
│   └── tests/
└── docs/superpowers/plans/
    └── 2026-05-04-rust-agent-platform-phase1.md
```

### Phase 1 Status

| Task | Status | Notes |
|------|--------|-------|
| 1. Project Scaffolding | ✅ Complete | Core types, Agent, Tool, Session, Provider |
| 2. Provider Abstraction | ✅ Complete | Multi-provider support |
| 3. Tool System | ✅ Complete | bash, read, write, edit, grep |
| 4. TUI Implementation | 🔄 WIP | Table rendering issues |
| 5. Intent Gate | ✅ Complete | Classification working |
| 6. Hook System | ✅ Complete | Hook registry |
| 7. Skill Loading | ✅ Complete | Skill registry |
| 8. MCP Integration | ✅ Complete | Client protocol |
| 9. Agent Orchestration | ✅ Complete | Scheduler, MessageBus |
| 10. Boulder Persistence | ✅ Complete | SQLite storage |

### Current Issues

#### TUI Table Rendering (Priority: HIGH)
**Problem:** Table headers not displaying correctly in markdown table rendering.

**Debug Info:**
- `headers.len()=1` - Header data detected correctly
- `rows.len()=2` - Data rows correct  
- Only 5 lines output instead of expected 6+

**Files:** `src/tui/app.rs` - `render_ascii_table()` function

**Next Steps:**
1. Add debug output to trace table rendering
2. Verify markdown parser table events firing correctly
3. Check border rendering logic

#### Provider API Inconsistency (Priority: MEDIUM)
Each provider has different constructor signatures - needs cleanup.

| Provider | Signature |
|----------|-----------|
| OpenAI | `(api_key, base_url)` |
| Anthropic | `(api_key, base_url)` |
| Ollama | `(base_url, model)` |
| DeepSeek | `(api_key, base_url, model)` |
| MiniMax | `(api_key, base_url, model)` |
| Custom | `(name, api_key, base_url, model)` |

### Tech Stack
- Agent Core: Modular Rust architecture
- Providers: OpenAI, Anthropic, Ollama, DeepSeek, MiniMax, Custom
- MCP: Custom protocol client
- TUI: ratatui 0.26 + crossterm 0.27
- Storage: rusqlite 0.31
- Markdown: pulldown-cmark 0.10

### Quick Start
```bash
cd rust-agent-platform
cargo run --bin ragent
```

### Configuration
- Config path: `~/.config/FastCode/config.json`
- Providers configured: OpenAI, Anthropic, Ollama, DeepSeek, MiniMax, Custom
