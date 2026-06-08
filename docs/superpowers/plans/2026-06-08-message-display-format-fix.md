# 消息显示格式修正计划 — Phase 1: user 消息移除 model 名

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 仅 user 消息的前缀去掉 model 名（`[user gpt-4o]:` → `[user]:`），assistant/system/error 保持 `[role model]:` 不变。

**Architecture:** `render_messages` 中格式化前缀时只对 `role == "user"` 特殊处理，省略 model 名。

**Tech Stack:** Rust, ratatui

---

## 相关文件

| 文件 | 改动 | 说明 |
|------|------|------|
| `swiftforge/src/tui/views/chat_view.rs:129` | 加 role 判断，仅 user 省略 model | 其他角色保持原格式 |

`chat_view.rs:142` streaming 前缀不动 — 保持 `[assistant model]: text▌`。

---

### Task 1: user 消息前缀去掉 model 名

**Files:**
- Modify: `swiftforge/src/tui/views/chat_view.rs:129`

- [ ] **Step 1: 改 `render_messages` 中 user 前缀格式**

修改 `chat_view.rs:129`：

```rust
// BEFORE:
let role_display = format!("[{} {}]", role, self.state.current_model);

// AFTER:
let role_display = match role.as_str() {
    "user" => format!("[{}]", role),
    _ => format!("[{} {}]", role, self.state.current_model),
};
```

- [ ] **Step 2: 验证编译**

```bash
cd swiftforge && cargo build --bin swiftforge 2>&1
```
预期：只出现已有的 4 个 warning，无 error。

- [ ] **Step 3: 运行测试验证**

```bash
cargo test --test tui_state_test --test task_coordinator_test 2>&1
```
预期：26/26 tests pass。

---

## 验证检查清单

- [ ] 用户消息显示为 `[user]: <text>`，而非 `[user gpt-4o]: <text>`
- [ ] 助手消息保持 `[assistant gpt-4o]: <text>` 不变
- [ ] streaming 前缀保持 `[assistant gpt-4o]: <text>▌` 不变
- [ ] `cargo build --bin swiftforge` 通过
- [ ] `cargo test --test tui_state_test` / `task_coordinator_test` 通过
