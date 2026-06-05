# SwiftForge 自动化测试框架设计

> 版本: 1.0
> 日期: 2026-06-04
> 状态: 设计中
> 类型: L3 设计/Spec

---

## 一、背景与目标

### 1.1 问题陈述

SwiftForge 需要一套自动化的测试机制来验证：
- **Agent 核心功能**: 对话、工具调用、状态管理
- **Provider 集成**: OpenAI、Anthropic、DeepSeek 等 LLM 提供商
- **编排系统**: TaskScheduler、MessageBus 多 Agent 协作
- **MCP 集成**: MCP 客户端与工具调用

### 1.2 设计目标

1. **脚本化测试**: 使用 YAML 定义测试场景，非工程师也可编写
2. **全面覆盖**: 覆盖单元、集成、E2E 各类测试
3. **真实环境**: 使用真实 API key 测试，与生产环境一致
4. **易于扩展**: 新的测试场景只需添加 YAML 文件
5. **详细报告**: 提供 JSON/HTML 等格式的测试报告

---

## 二、整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        Test Runner (CLI)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   scripts/   │  │    runner/   │  │       report/        │  │
│  │              │  │              │  │                      │  │
│  │ *.yaml       │  │ parser.rs    │  │ json.rs              │  │
│  │ (Test Cases) │─▶│ executor.rs  │─▶│ html.rs              │  │
│  │              │  │ verifier.rs  │  │ junit.rs (CI兼容)    │  │
│  │              │  │ context.rs   │  │                      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│                          │                                      │
└──────────────────────────┼──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Existing Infrastructure                      │
│                                                                  │
│  ┌────────┐ ┌──────────┐ ┌───────┐ ┌────────┐ ┌─────────────┐ │
│  │ Agent  │ │ Provider │ │ Tools │ │Session │ │TaskScheduler│ │
│  └────────┘ └──────────┘ └───────┘ └────────┘ └─────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.1 组件职责

| 组件 | 职责 |
|------|------|
| `scripts/` | YAML 测试脚本存储目录 |
| `runner/parser.rs` | 解析 YAML 脚本为内部结构 |
| `runner/executor.rs` | 执行测试步骤，调用实际系统 |
| `runner/verifier.rs` | 验证期望与实际结果 |
| `runner/context.rs` | 维护测试执行状态 |
| `report/` | 生成各类测试报告 |

---

## 三、YAML 脚本格式规范

### 3.1 完整结构

```yaml
# ===========================================
# SwiftForge Test Script Specification v1.0
# ===========================================

# --- 元信息 (必填) ---
name: "测试场景名称"
description: "详细描述这个测试场景验证的功能"
version: "1.0"
tags:
  - "agent"
  - "e2e"
  - "openai"

# --- 配置 (必填) ---
config:
  # Provider 配置
  provider:
    type: "openai"                      # openai | anthropic | deepseek | ollama | minimax | custom
    api_key: "${OPENAI_API_KEY}"         # 环境变量引用
    base_url: "${OPENAI_BASE_URL}"       # 可选，默认使用官方地址
    model: "gpt-4o"                      # 模型名称
    temperature: 0.7                     # 可选，默认 0.0
    
  # Agent 配置  
  agent:
    name: "test-agent"
    role: "Executor"                     # Executor | Orchestrator | Planner | Advisor | Explorer | Librarian
    tools:
      - "bash"
      - "read"
      - "write"
      - "edit"
      - "grep"
      
  # 测试环境配置
  environment:
    timeout_ms: 30000                    # 单步超时
    retry_count: 2                       # 失败重试次数
    retry_delay_ms: 1000                 # 重试间隔

# --- 测试步骤 (必填，至少一步) ---
steps:
  - name: "step_name"
    description: "步骤描述"
    action: "chat"                       # chat | tool_call | assert | wait | loop | condition | snapshot
    # ... 更多字段取决于 action 类型
```

### 3.2 Action 类型详解

#### 3.2.1 `chat` - 对话测试

```yaml
- name: "basic_chat"
  action: "chat"
  input:
    messages:
      - role: "system"
        content: "You are a helpful assistant."
      - role: "user"  
        content: "What is 2 + 2? Only answer with the number."
  expect:
    response_success: true
    response_contains: "4"
    response_time_ms: 5000              # 可选，期望最大响应时间
```

#### 3.2.2 `tool_call` - 工具调用

```yaml
- name: "execute_bash"
  action: "tool_call"
  tool: "bash"
  arguments:
    command: "echo 'hello world'"
  expect:
    success: true
    output_contains: "hello world"
    stderr_empty: true
```

#### 3.2.3 `assert` - 条件断言

```yaml
- name: "verify_state"
  action: "assert"
  conditions:
    - "last_response.success == true"
    - "last_response.content_length > 0"
    - "tool_results[0].success == true"
```

#### 3.2.4 `wait` - 等待

```yaml
- name: "wait_for_processing"
  action: "wait"
  duration_ms: 500
```

#### 3.2.5 `loop` - 循环测试

```yaml
- name: "multi_turn_conversation"
  action: "loop"
  times: 3
  steps:
    - action: "chat"
      input:
        messages:
          - role: "user"
            content: "Reply with just the word 'loop'"
      expect:
        response_contains: "loop"
```

#### 3.2.6 `condition` - 条件分支

```yaml
- name: "conditional_test"
  action: "condition"
  condition: "response.content contains 'error'"
  then:
    - action: "assert"
      conditions:
        - "response.success == false"
  else:
    - action: "assert"
      conditions:
        - "response.success == true"
```

#### 3.2.7 `snapshot` - 状态快照

```yaml
- name: "save_state"
  action: "snapshot"
  name: "after_chat"
  data:
    response_length: "response.content.length"
    tool_count: "len(tool_results)"
```

### 3.3 `verify` - 最终状态验证

```yaml
verify:
  final_state:
    messages_count: 4                   # 期望的消息总数
    last_role: "assistant"              # 最后一条消息的角色
    tools_called: ["bash"]              # 调用过的工具列表
    
  snapshots:
    - name: "after_first_chat"
      data:
        response_length: "response.content.length"
        tool_count: "len(tool_results)"
```

### 3.4 环境变量

```yaml
config:
  provider:
    api_key: "${OPENAI_API_KEY}"        # 从环境变量读取
    base_url: "${OPENAI_BASE_URL:-https://api.openai.com/v1}"  # 支持默认值
```

### 3.5 示例脚本

#### agent_basic_chat.yaml

```yaml
name: "Agent Basic Chat Test"
description: "测试 Agent 与 OpenAI Provider 的基本对话功能"
version: "1.0"
tags: ["agent", "chat", "openai"]

config:
  provider:
    type: "openai"
    api_key: "${OPENAI_API_KEY}"
    model: "gpt-4o"
    temperature: 0.0
    
  agent:
    name: "test-agent"
    role: "Executor"
    tools: []
    
  environment:
    timeout_ms: 30000
    retry_count: 2

steps:
  - name: "simple_chat"
    action: "chat"
    input:
      messages:
        - role: "user"
          content: "What is 2 + 2? Answer with just the number."
    expect:
      response_success: true
      response_contains: "4"
      response_time_ms: 10000

  - name: "verify_empty_response"
    action: "assert"
    conditions:
      - "last_response.success == true"
      - "len(last_response.content) > 0"
```

#### agent_tool_execution.yaml

```yaml
name: "Agent Tool Execution Test"
description: "测试 Agent 执行 bash 工具的能力"
version: "1.0"
tags: ["agent", "tool", "bash"]

config:
  provider:
    type: "openai"
    api_key: "${OPENAI_API_KEY}"
    model: "gpt-4o"
    temperature: 0.0
    
  agent:
    name: "test-agent"
    role: "Executor"
    tools: ["bash"]
    
  environment:
    timeout_ms: 30000

steps:
  - name: "test_bash_echo"
    action: "tool_call"
    tool: "bash"
    arguments:
      command: "echo 'hello world'"
    expect:
      success: true
      output_contains: "hello world"

  - name: "test_bash_pwd"
    action: "tool_call"
    tool: "bash"
    arguments:
      command: "pwd"
    expect:
      success: true
      output_contains: "/"
```

#### provider_openai_streaming.yaml

```yaml
name: "OpenAI Streaming Test"
description: "测试 OpenAI 流式响应功能"
version: "1.0"
tags: ["provider", "streaming", "openai"]

config:
  provider:
    type: "openai"
    api_key: "${OPENAI_API_KEY}"
    model: "gpt-4o"
    
  agent:
    name: "test-agent"
    role: "Executor"
    tools: []
    
  environment:
    timeout_ms: 30000

steps:
  - name: "streaming_chat"
    action: "chat"
    input:
      messages:
        - role: "user"
          content: "Count from 1 to 3, one number per response."
    expect:
      response_success: true
      response_contains: "1"
      response_time_ms: 8000
```

---

## 四、核心数据结构

### 4.1 Rust 类型定义

```rust
// runner/src/context.rs

/// 测试上下文 - 维护测试执行状态
pub struct TestContext {
    /// 当前 Agent 实例
    pub agent: Option<Agent>,
    /// 对话历史消息
    pub messages: Vec<Message>,
    /// 工具执行结果
    pub tool_results: Vec<ToolResult>,
    /// 快照数据
    pub snapshots: HashMap<String, Value>,
    /// 脚本内变量
    pub variables: HashMap<String, Value>,
    /// 环境变量
    pub env: HashMap<String, String>,
}

impl TestContext {
    pub fn new(env: HashMap<String, String>) -> Self;
    pub fn set_response(&mut self, response: ModelResponse);
    pub fn add_message(&mut self, role: &str, content: &str);
    pub fn add_tool_result(&mut self, result: ToolResult);
    pub fn save_snapshot(&mut self, name: &str, data: Value);
}
```

```rust
// runner/src/parser.rs

/// 测试脚本结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScript {
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub config: ScriptConfig,
    pub steps: Vec<TestStep>,
    pub verify: Option<VerifyConfig>,
}

/// 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    pub provider: ProviderConfig,
    pub agent: AgentConfig,
    pub environment: EnvironmentConfig,
}

/// 测试步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TestStep {
    Chat {
        name: String,
        description: Option<String>,
        input: ChatInput,
        expect: ChatExpect,
    },
    ToolCall {
        name: String,
        description: Option<String>,
        tool: String,
        arguments: HashMap<String, serde_json::Value>,
        expect: ToolExpect,
    },
    Assert {
        name: String,
        description: Option<String>,
        conditions: Vec<String>,
    },
    Wait {
        name: String,
        description: Option<String>,
        duration_ms: u64,
    },
    Loop {
        name: String,
        description: Option<String>,
        times: usize,
        steps: Vec<TestStep>,
    },
    Condition {
        name: String,
        description: Option<String>,
        condition: String,
        then: Vec<TestStep>,
        else: Option<Vec<TestStep>>,
    },
    Snapshot {
        name: String,
        description: Option<String>,
        snapshot_name: String,
        data: HashMap<String, String>,
    },
}
```

```rust
// runner/src/executor.rs

/// 测试执行器
pub struct TestExecutor {
    script: TestScript,
    context: TestContext,
    reporters: Vec<Box<dyn Reporter>>,
}

impl TestExecutor {
    /// 执行整个测试脚本
    pub async fn run(&mut self) -> Result<TestSummary> {
        for reporter in &mut self.reporters {
            reporter.on_test_start(&self.script);
        }
        
        for step in &self.script.steps {
            let result = self.execute_step(step).await?;
            
            for reporter in &mut self.reporters {
                reporter.on_step_complete(&result);
            }
            
            if !result.success {
                break;
            }
        }
        
        let summary = self.build_summary();
        for reporter in &mut self.reporters {
            reporter.on_test_complete(&summary);
        }
        
        Ok(summary)
    }
    
    /// 执行单个步骤
    async fn execute_step(&mut self, step: &TestStep) -> Result<StepResult> {
        match step {
            TestStep::Chat { name, input, expect, .. } => {
                self.execute_chat(name, input, expect).await
            }
            TestStep::ToolCall { name, tool, arguments, expect, .. } => {
                self.execute_tool_call(name, tool, arguments, expect).await
            }
            TestStep::Assert { name, conditions, .. } => {
                self.execute_assert(name, conditions).await
            }
            TestStep::Wait { name, duration_ms, .. } => {
                self.execute_wait(name, *duration_ms).await
            }
            TestStep::Loop { name, times, steps, .. } => {
                self.execute_loop(name, *times, steps).await
            }
            TestStep::Condition { name, condition, then, else, .. } => {
                self.execute_condition(name, condition, then, else.as_deref()).await
            }
            TestStep::Snapshot { name, snapshot_name, data, .. } => {
                self.execute_snapshot(name, snapshot_name, data).await
            }
        }
    }
}
```

```rust
// runner/src/verifier.rs

/// 验证器
pub struct Verifier;

impl Verifier {
    /// 验证 chat 期望
    pub fn verify_chat_expect(
        response: &ModelResponse,
        expect: &ChatExpect,
    ) -> Result<()> {
        if expect.response_success && !response.content.contains(&expect.response_contains.unwrap_or_default()) {
            return Err(anyhow!("Response does not contain expected text"));
        }
        Ok(())
    }
    
    /// 验证工具调用期望
    pub fn verify_tool_expect(
        result: &ToolResult,
        expect: &ToolExpect,
    ) -> Result<()> {
        if expect.success != result.success {
            return Err(anyhow!("Tool success mismatch"));
        }
        // ... 更多验证
        Ok(())
    }
    
    /// 解析并执行条件表达式
    pub fn evaluate_condition(condition: &str, context: &TestContext) -> bool {
        // 实现简单的表达式求值
        // 例如: "response.content contains 'error'"
    }
}
```

```rust
// runner/src/reporters/mod.rs

/// 报告器接口
pub trait Reporter: Send + Sync {
    fn on_test_start(&mut self, script: &TestScript);
    fn on_step_start(&mut self, step: &TestStep);
    fn on_step_complete(&mut self, result: &StepResult);
    fn on_test_complete(&mut self, summary: &TestSummary);
}

/// JSON 报告器
pub struct JsonReporter {
    output_path: PathBuf,
}

/// HTML 报告器
pub struct HtmlReporter {
    output_path: PathBuf,
}

/// JUnit XML 报告器 (CI 兼容)
pub struct JUnitReporter {
    output_path: PathBuf,
}
```

---

## 五、CLI 接口

### 5.1 命令行接口

```bash
# 运行单个脚本
cargo test --script scripts/agent/basic_chat.yaml

# 运行目录下的所有脚本
cargo test --script-dir scripts/

# 带详细输出
cargo test --script scripts/agent/basic_chat.yaml --verbose

# 生成 HTML 报告
cargo test --script scripts/agent/basic_chat.yaml \
    --report html \
    --output test-report.html

# 生成 JSON 报告
cargo test --script scripts/agent/basic_chat.yaml \
    --report json \
    --output test-report.json

# 生成 JUnit XML (CI)
cargo test --script scripts/agent/basic_chat.yaml \
    --report junit \
    --output test-results.xml

# 并行运行多个脚本
cargo test --script-dir scripts/ --parallel 4

# 过滤标签
cargo test --script-dir scripts/ --tags agent,openai

# 设置环境变量
OPENAI_API_KEY=sk-xxx cargo test --script scripts/agent/basic_chat.yaml
```

### 5.2 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 所有测试通过 |
| 1 | 部分或全部测试失败 |
| 2 | 测试执行出错 (解析错误、超时等) |

---

## 六、目录结构

```
swiftforge/tests/
├── scripts/
│   ├── agent/
│   │   ├── basic_chat.yaml
│   │   ├── multi_turn_conversation.yaml
│   │   ├── tool_execution.yaml
│   │   └── intent_classification.yaml
│   ├── provider/
│   │   ├── openai_chat.yaml
│   │   ├── anthropic_claude.yaml
│   │   ├── deepseek.yaml
│   │   └── ollama_local.yaml
│   └── orchestration/
│       ├── multi_agent_coordination.yaml
│       └── task_scheduler.yaml
├── runner/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── parser.rs
│   │   ├── executor.rs
│   │   ├── verifier.rs
│   │   ├── context.rs
│   │   └── reporters/
│   │       ├── mod.rs
│   │       ├── json.rs
│   │       ├── html.rs
│   │       └── junit.rs
│   └── Cargo.toml
├── Cargo.toml                    # 修改，添加测试 runner 依赖
└── lib.rs
```

---

## 七、测试报告格式

### 7.1 JSON 报告

```json
{
  "test_name": "Agent Basic Chat Test",
  "status": "passed",
  "duration_ms": 1523,
  "timestamp": "2026-06-04T10:30:00Z",
  "config": {
    "provider": {
      "type": "openai",
      "model": "gpt-4o"
    },
    "agent": {
      "name": "test-agent",
      "role": "Executor"
    }
  },
  "steps": [
    {
      "name": "simple_chat",
      "action": "chat",
      "status": "passed",
      "duration_ms": 1234,
      "response": {
        "content": "4",
        "success": true
      }
    },
    {
      "name": "verify_empty_response",
      "action": "assert",
      "status": "passed",
      "duration_ms": 5,
      "conditions": [
        "last_response.success == true",
        "len(last_response.content) > 0"
      ]
    }
  ],
  "summary": {
    "total_steps": 2,
    "passed_steps": 2,
    "failed_steps": 0,
    "skipped_steps": 0
  }
}
```

### 7.2 JUnit XML

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="Agent Basic Chat Test" tests="2" failures="0" time="1.523">
  <testcase name="simple_chat" classname="agent" time="1.234">
    <system-out>Response: 4</system-out>
  </testcase>
  <testcase name="verify_empty_response" classname="agent" time="0.005"/>
</testsuite>
```

---

## 八、实现计划

### Phase 1: 核心框架

1. **目录结构创建**
   - 创建 `tests/runner/` 目录
   - 创建 `tests/scripts/` 目录

2. **解析器实现**
   - 实现 `TestScript` 结构
   - 实现 YAML 解析逻辑
   - 支持环境变量替换

3. **执行器实现**
   - 实现 `TestExecutor`
   - 实现各 `TestStep` 执行逻辑
   - 支持重试机制

4. **验证器实现**
   - 实现期望验证逻辑
   - 实现条件表达式求值

### Phase 2: 报告系统

5. **JSON 报告器**
6. **HTML 报告器**
7. **JUnit 报告器**

### Phase 3: 测试脚本

8. **示例脚本编写**
   - Agent 基本测试
   - Provider 测试
   - 工具执行测试

### Phase 4: CLI 集成

9. **命令行接口**
   - 参数解析
   - 报告生成

---

## 九、依赖

```toml
# tests/runner/Cargo.toml
[dependencies]
# 现有依赖
swiftforge = { path = ".." }
swiftforge-types = { path = "../../libs/swiftforge-types" }
swiftforge-providers = { path = "../../libs/swiftforge-providers" }

# 新增依赖
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tempfile = "3"
chrono = "0.4"

[dev-dependencies]
# 用于集成测试
mockito = "1"
```