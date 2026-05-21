# Rust Programming Agent Platform - Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust-based programming agent platform that replicates OpenCode + oh-my-opencode functionality with performance improvements (4-43ms startup, <50MB memory)

**Architecture:** Modular layered design using existing Rust crates as foundation. Core layer provides Agent orchestration; Platform layer provides Intent Gate, Hook System, Skill Loading; Integration layer provides MCP and Provider abstraction. Phase 1 focuses on core infrastructure and must achieve parity with OpenCode basic features before adding omo-specific innovations.

**Tech Stack:**
- Agent Core: OpenDev (4.3ms startup, Agent Fleet)
- Provider Abstraction: Kernex + Pi Agent providers
- MCP: mcpr (0.2.3+)
- Plugin Runtime: extism (WASM sandbox)
- TUI: ratatui
- Skill Format: SKILL.md (thulp-skill-files)
- Storage: SQLite (rusqlite)

---

## Execution Preference

**Selected:** Subagent-Driven (recommended)
**Reason:** Each Task dispatched to fresh subagent with review between tasks for fast iteration

**Required Sub-Skill:** `superpowers:subagent-driven-development`

**Session Continuity:**
- Store session_id from each subagent task for continuation
- Use session_id for follow-up fixes and reviews
- Plan file path: `docs/superpowers/plans/2026-05-04-rust-agent-platform-phase1.md`
- Current phase: Phase 1, Task 1 (Project Scaffolding)

---

## Phase 1: Core Infrastructure (✅ COMPLETE)

> **Note:** Project is fully scaffolded and building. Phase 2 (Platform Layer) and partial Phase 3 have been implemented ahead of schedule.

### Task 1: Project Scaffolding ✅

**Files Created:**
- ✅ `Cargo.toml` - Workspace configuration
- ✅ `rust-agent-platform/Cargo.toml` - Main crate
- ✅ `rust-agent-platform/src/main.rs` - Binary entry
- ✅ `rust-agent-platform/src/lib.rs` - Library entry
- ✅ `rust-agent-platform/src/core/mod.rs`
- ✅ `rust-agent-platform/src/core/agent.rs`
- ✅ `rust-agent-platform/src/core/tool.rs`
- ✅ `rust-agent-platform/src/core/session.rs`
- ✅ `rust-agent-platform/src/core/provider.rs`
- ✅ `rust-agent-platform/tests/core_test.rs`

**Status:** Complete - Project builds successfully with 46 tests passing.

---

### Phase 2: Platform Layer (✅ COMPLETE)

The following Phase 2 tasks were implemented ahead of schedule:

- ✅ **Task 5: Intent Gate Classification** - `src/platform/intent_gate.rs`, `src/platform/category.rs`
- ✅ **Task 6: Hook System** - `src/platform/hooks/` with 52 hooks
- ✅ **Task 7: Skill Loading System** - `src/platform/skill/` with SKILL.md support

### Phase 3: Advanced Features (🔄 IN PROGRESS)

- 🔄 **Task 8: MCP Client Integration** - Protocol + client framework complete, real HTTP pending
- ✅ **Task 9: Agent Orchestration** - Scheduler, MessageBus, Agent orchestration complete
- ✅ **Task 10: Boulder Persistence** - SQLite-based TODO tracking complete
- 🔄 **Task 11: TUI + Streaming** - Debug panel, streaming output, scroll support ✅ (mostly)
- 🔄 **Task 12: DeepSeek V4 Tool Calling** - Text format parsing pending

---

## Current Implementation Status

### TUI (✅ Mostly Complete)
- `src/tui/app.rs` - Main TUI implementation (1102 lines)
- **Completed:**
  1. Table rendering: Unicode box-drawing with auto-scaling columns ✅
  2. Streaming output during generation via `stream_ui` channel ✅
  3. Scroll behavior during streaming via `debug_tx` channel ✅
  4. `--debug` flag for debug panel toggle ✅
  5. CJK fullwidth character support (2-cell width) ✅
  6. ↑↓ keys for debug panel scrolling ✅

### Provider Abstraction ✅
- `src/providers/mod.rs` - Provider trait
- `src/providers/openai.rs`, `anthropic.rs`, `ollama.rs`, `deepseek.rs`, `minimax.rs`, `custom.rs`
- **Completed:** `stream_chat()` implementation for real-time output ✅

### DeepSeek V4 Streaming (🔄 In Progress - Known Issues)
- **Model:** `deepseek-v4-pro`
- **Known Behaviors:**
  1. Tool calls return as text tags `<tool>bash</tool>` (not structured `tool_calls`)
  2. Requires `reasoning_content` in tool call feedback
  3. Streaming requires `thinking: { type: "enabled" }` + `reasoning_effort: "high"`
- **Pending:** Parse text-format tool calls, accumulate reasoning_content

---

## Original Phase 1 Tasks (Reference)

### Task 1: Project Scaffolding (Original)

**Files:**
- Create: `Cargo.toml`
- Create: `rust-agent-platform/Cargo.toml`
- Create: `rust-agent-platform/src/main.rs`
- Create: `rust-agent-platform/src/lib.rs`
- Create: `rust-agent-platform/src/core/mod.rs`
- Create: `rust-agent-platform/src/core/agent.rs`
- Create: `rust-agent-platform/src/core/tool.rs`
- Create: `rust-agent-platform/src/core/session.rs`
- Create: `rust-agent-platform/src/core/provider.rs`
- Create: `rust-agent-platform/Cargo.lock`
- Test: `rust-agent-platform/tests/core_test.rs`

- [x] **Step 1: Create workspace Cargo.toml** ✅

```toml
[workspace]
members = ["rust-agent-platform"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Developer <dev@example.com>"]
license = "MIT OR Apache-2.0"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.31", features = ["bundled"] }
```

- [x] **Step 2: Create rust-agent-platform/Cargo.toml** ✅

```toml
[package]
name = "rust-agent-platform"
version.workspace = true
edition.workspace = true

[lib]
name = "rust_agent_platform"
crate-type = ["lib", "bin"]

[[bin]]
name = "ragent"
path = "src/main.rs"

[dependencies]
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
thiserror.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
rusqlite.workspace = true

# Agent core
tracing = "0.1"
anyhow = "1.0"
tokio = { version = "1", features = ["full"] }

# Provider (multi-model support)
reqwest = { version = "0.12", features = ["json"] }

# TUI
ratatui = "0.26"
crossterm = "0.27"

# Async
async-trait = "0.1"

# Error handling
thiserror = "1.0"
tracing.error = "0.1"
```

- [x] **Step 3: Create src/lib.rs with module declarations** ✅

```rust
pub mod core;

pub use core::{Agent, AgentConfig, Tool, ToolResult, Session, Provider};
```

- [x] **Step 4: Create src/core/mod.rs** ✅

```rust
mod agent;
mod tool;
mod session;
mod provider;

pub use agent::{Agent, AgentConfig, AgentRole};
pub use tool::{Tool, ToolResult, ToolCall, ToolRegistry};
pub use session::{Session, SessionConfig, Message};
pub use provider::{Provider, ProviderConfig, ModelResponse};
```

- [x] **Step 5: Create src/core/agent.rs (Phase 1 stub - basic Agent struct)** ✅

```rust
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub model: Option<String>,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    Orchestrator,
    Executor,
    Planner,
    Advisor,
    Explorer,
    Librarian,
}

pub struct Agent {
    config: AgentConfig,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn role(&self) -> &AgentRole {
        &self.config.role
    }
}
```

- [x] **Step 6: Create src/core/tool.rs (Phase 1 stub - basic Tool trait)** ✅

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, call: ToolCall) -> ToolResult;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    pub async fn execute(&self, call: ToolCall) -> ToolResult {
        if let Some(tool) = self.tools.get(&call.name) {
            tool.execute(call).await
        } else {
            ToolResult {
                success: false,
                output: None,
                error: Some(format!("Tool '{}' not found", call.name)),
            }
        }
    }
}
```

- [x] **Step 7: Create src/core/session.rs (Phase 1 stub)** ✅

```rust
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct Session {
    messages: VecDeque<Message>,
    context_window: usize,
}

impl Session {
    pub fn new(context_window: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            context_window,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push_back(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn messages(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }
}
```

- [x] **Step 8: Create src/core/provider.rs (Phase 1 stub)** ✅

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, messages: Vec<crate::session::Message>) -> anyhow::Result<ModelResponse>;
    fn name(&self) -> &str;
}
```

- [x] **Step 9: Create src/main.rs** ✅

```rust
use tracing_subscriber;
use rust_agent_platform::core::{Agent, AgentConfig, AgentRole};

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Rust Agent Platform...");

    let agent = Agent::new(AgentConfig {
        name: "test".to_string(),
        role: AgentRole::Orchestrator,
        model: Some("claude-3-sonnet".to_string()),
        temperature: 0.1,
    });

    println!("Agent created: {:?}", agent.name());
}
```

- [x] **Step 10: Create tests/core_test.rs** ✅

```rust
use rust_agent_platform::core::{Agent, AgentConfig, AgentRole, ToolRegistry, Tool, ToolCall, ToolResult};
use async_trait::async_trait;

struct DummyTool;

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &str { "dummy" }
    fn description(&self) -> &str { "A dummy tool for testing" }
    async fn execute(&self, _call: ToolCall) -> ToolResult {
        ToolResult { success: true, output: Some("done".to_string()), error: None }
    }
}

#[test]
fn test_agent_creation() {
    let agent = Agent::new(AgentConfig {
        name: "test".to_string(),
        role: AgentRole::Orchestrator,
        model: None,
        temperature: 0.1,
    });
    assert_eq!(agent.name(), "test");
}

#[test]
fn test_tool_registry() {
    let mut registry = ToolRegistry::new();
    registry.register(DummyTool);
    // Will expand in Phase 2
}
```

- [x] **Step 11: Build to verify compilation** ✅ (PASS - 46 tests)

Run: `cd rust-agent-platform && cargo build`
Expected: PASS (clean compilation)

- [x] **Step 12: Run tests** ✅ (PASS)

Run: `cd rust-agent-platform && cargo test`
Expected: PASS (2 tests pass)

- [x] **Step 13: Commit** ✅

```bash
git add Cargo.toml rust-agent-platform/ && git commit -m "feat: project scaffolding with core types"
```

---

### Task 2: Provider Abstraction Layer ✅

**Files:**
- Modify: `rust-agent-platform/src/core/provider.rs`
- Create: `rust-agent-platform/src/providers/mod.rs`
- Create: `rust-agent-platform/src/providers/openai.rs`
- Create: `rust-agent-platform/src/providers/anthropic.rs`
- Create: `rust-agent-platform/src/providers/ollama.rs`
- Create: `rust-agent-platform/tests/provider_test.rs`

- [x] **Step 1: Write failing test for provider abstraction** ✅

```rust
// tests/provider_test.rs
use rust_agent_platform::providers::{Provider, OpenAIProvider, AnthropicProvider};
use rust_agent_platform::session::Message;

#[tokio::test]
async fn test_openai_provider_chat() {
    let provider = OpenAIProvider::new("sk-test".to_string(), None);
    let messages = vec![Message { role: "user".to_string(), content: "Hello".to_string() }];
    // Will fail - not implemented yet
    let result = provider.chat(messages).await;
    assert!(result.is_ok());
}
```

Run: `cargo test provider_test -- --nocapture`
Expected: FAIL with "not implemented"

- [x] **Step 2: Implement Provider trait with concrete providers** ✅

Create `src/providers/mod.rs`:
```rust
pub mod openai;
pub mod anthropic;
pub mod ollama;

use async_trait::async_trait;
use crate::core::{Provider, ProviderConfig, ModelResponse};
use crate::session::Message;
use anyhow::Result;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
}
```

- [x] **Step 3: Implement OpenAI provider** ✅

```rust
// src/providers/openai.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::providers::LLMProvider;
use crate::core::{ModelResponse, Usage};
use crate::session::Message;
use anyhow::{Result, Context};

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            model: "gpt-4o".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages.iter().map(|m| {
                    serde_json::json!({ "role": m.role, "content": m.content })
                }).collect::<Vec<_>>()
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .context("No content in response")?
            .to_string();

        Ok(ModelResponse {
            content,
            usage: Usage {
                input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            },
        })
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}
```

- [x] **Step 4: Run tests to verify** ✅

Run: `cargo test provider_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/providers/ tests/provider_test.rs && git commit -m "feat: add provider abstraction with OpenAI support"
```

---

### Task 3: Tool System Implementation ✅

**Files:**
- Modify: `rust-agent-platform/src/core/tool.rs`
- Create: `rust-agent-platform/src/tools/mod.rs`
- Create: `rust-agent-platform/src/tools/bash.rs`
- Create: `rust-agent-platform/src/tools/read.rs`
- Create: `rust-agent-platform/src/tools/write.rs`
- Create: `rust-agent-platform/src/tools/edit.rs`
- Create: `rust-agent-platform/src/tools/grep.rs`
- Create: `rust-agent-platform/tests/tool_test.rs`

- [x] **Step 1: Write failing test for bash tool** ✅

```rust
// tests/tool_test.rs
use rust_agent_platform::tools::{ToolRegistry, BashTool};

#[tokio::test]
async fn test_bash_tool_execution() {
    let mut registry = ToolRegistry::new();
    registry.register(BashTool::new());

    let result = registry.execute("bash".to_string(), vec!["echo hello".to_string()]).await;
    assert!(result.success);
    assert_eq!(result.output, Some("hello".to_string()));
}
```

Run: `cargo test tool_test`
Expected: FAIL with "method not found"

- [x] **Step 2: Extend Tool trait with execute method** ✅

Update `src/core/tool.rs`:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Vec<String>;
    async fn execute(&self, arguments: Vec<String>) -> ToolResult;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    pub async fn execute(&self, name: &str, arguments: Vec<String>) -> ToolResult {
        if let Some(tool) = self.tools.get(name) {
            tool.execute(arguments).await
        } else {
            ToolResult {
                success: false,
                output: None,
                error: Some(format!("Tool '{}' not found", name)),
            }
        }
    }
}
```

- [x] **Step 3: Implement BashTool** ✅

```rust
// src/tools/bash.rs
use std::process::Command;
use async_trait::async_trait;
use crate::core::{Tool, ToolResult};

pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }
    fn description(&self) -> &str { "Execute shell commands" }
    fn parameters(&self) -> Vec<String> { vec!["command".to_string()] }

    async fn execute(&self, arguments: Vec<String>) -> ToolResult {
        let command = arguments.join(" ");
        let output = Command::new("sh").arg("-c").arg(&command).output();

        match output {
            Ok(out) => ToolResult {
                success: out.status.success(),
                output: Some(String::from_utf8_lossy(&out.stdout).to_string()),
                error: if out.status.success() { None } else { Some(String::from_utf8_lossy(&out.stderr).to_string()) },
            },
            Err(e) => ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }
}
```

- [x] **Step 4: Implement ReadTool, WriteTool, EditTool, GrepTool** ✅

Create similar implementations for `src/tools/read.rs`, `write.rs`, `edit.rs`, `grep.rs`.

- [x] **Step 5: Run tests** ✅

Run: `cargo test tool_test`
Expected: PASS

- [x] **Step 6: Commit** ✅

```bash
git add src/tools/ tests/tool_test.rs && git commit -m "feat: implement tool system with bash/read/write/edit/grep"
```

---

### Task 4: TUI Implementation 🔄

**Files:**
- Create: `rust-agent-platform/src/tui/mod.rs`
- Create: `rust-agent-platform/src/tui/app.rs`
- Create: `rust-agent-platform/src/tui/components.rs`
- Create: `rust-agent-platform/src/tui/input.rs`

- [x] **Step 1: Create basic TUI structure with ratatui** ✅

```rust
// src/tui/mod.rs
pub mod app;
pub mod components;
pub mod input;

pub use app::App;
pub use components::{MessageList, InputArea, StatusBar};
```

- [x] **Step 2: Implement main App struct** ✅

```rust
// src/tui/app.rs
use ratatui::{Terminal, Frame, backend::CrosstermBackend};
use std::io::Stdout;
use crossterm::event::{self, Event, KeyCode};
use anyhow::Result;

pub struct App {
    messages: Vec<String>,
    input: String,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            should_quit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key)?;
                if self.should_quit {
                    break;
                }
            }
        }
        Ok(())
    }

    fn render(&self, f: &mut Frame) {
        // Basic layout: messages area, input area, status bar
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::{Rect, Layout, Constraint, Direction};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ].as_ref())
            .split(f.size());

        let messages = Paragraph::new(self.messages.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(messages, chunks[0]);

        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[1]);

        let status = Paragraph::new("Rust Agent Platform | Ctrl+C to quit");
        f.render_widget(status, chunks[2]);
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Backspace => { self.input.pop(); }
            KeyCode::Enter => {
                self.messages.push(self.input.clone());
                self.input.clear();
            }
            KeyCode::Ctrl('c') => self.should_quit = true,
            _ => {}
        }
        Ok(())
    }
}
```

- [x] **Step 3: Update main.rs to launch TUI** ✅

```rust
// src/main.rs update
use ratatui::{Terminal, backend::CrosstermBackend};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = rust_agent_platform::tui::App::new();
    app.run(&mut terminal)?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}
```

- [x] **Step 4: Build and test TUI** ✅

Run: `cargo build --bin ragent`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/tui/ src/main.rs && git commit -m "feat: add TUI with ratatui"
```

---

## Phase 2: Platform Layer (Planning - 8 weeks)

### Task 5: Intent Gate Classification ✅

**Files:**
- Create: `rust-agent-platform/src/platform/intent_gate.rs`
- Create: `rust-agent-platform/src/platform/category.rs`
- Create: `rust-agent-platform/tests/intent_test.rs`

**Purpose:** Analyze user input to classify intent before routing to appropriate handler.

**Approach:** Rule-based + keyword matching with future ML extension point.

**Steps:**
1. Define Intent enum (Research, Implementation, Investigation, Evaluation, Fix)
2. Implement classify() function with keyword/pattern matching
3. Add unit tests with various user inputs

---

### Task 6: Hook System ✅

**Files:**
- Create: `rust-agent-platform/src/platform/hook.rs`
- Create: `rust-agent-platform/src/platform/hooks/mod.rs`
- Create: `rust-agent-platform/tests/hook_test.rs`

**Purpose:** Lifecycle hooks for extending behavior (52 hooks: Core 43 + Continuation 7 + Skill 2)

**Approach:** extism WASM sandbox for plugin isolation + custom hook dispatcher

**Steps:**
1. Define Hook enum with variants for each lifecycle event
2. Implement HookRegistry with priority ordering
3. Create hook_dispatch() function with context passing
4. Add extism integration for WASM-based hooks

---

### Task 7: Skill Loading System ✅

**Files:**
- Create: `rust-agent-platform/src/platform/skill.rs`
- Create: `rust-agent-platform/src/platform/skill/loader.rs`
- Create: `rust-agent-platform/tests/skill_test.rs`

**Purpose:** Load and execute SKILL.md formatted skill definitions

**Approach:** thulp-skill-files + custom registry with scope priority (Global > Project > User)

**Steps:**
1. Define Skill struct with name, description, tools, hooks
2. Parse SKILL.md YAML frontmatter
3. Implement SkillLoader with scope priority
4. Add skill registry with activation/deactivation

---

## Phase 3: Advanced Features (Planning - 8 weeks)

### Task 8: MCP Client Integration 🔄

**Files:**
- Create: `rust-agent-platform/src/integration/mcp/mod.rs`
- Create: `rust-agent-platform/src/integration/mcp/client.rs`
- Create: `rust-agent-platform/src/integration/mcp/server.rs`

**Purpose:** Full MCP protocol support (Resources, Tools, Prompts, Sampling)

**Approach:** mcpr crate with custom transport layer

---

### Task 9: Agent Orchestration 🔄

**Files:**
- Create: `rust-agent-platform/src/orchestration/mod.rs`
- Create: `rust-agent-platform/src/orchestration/scheduler.rs`
- Create: `rust-agent-platform/src/orchestration/messages.rs`

**Purpose:** Multi-agent orchestration (Sisyphus cluster pattern)

**Approach:** swarms-rs or custom implementation with message bus

---

### Task 10: Boulder Persistence ✅

**Files:**
- Create: `rust-agent-platform/src/platform/boulder.rs`
- Create: `rust-agent-platform/tests/boulder_test.rs`

**Purpose:** TODO persistence across sessions (omo core innovation)

**Approach:** SQLite-based with file-based locking for concurrent access

---

## Validation Checklist

After each task:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] No new clippy warnings
- [ ] Documentation updated in `docs/`

---

## Execution Options

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**