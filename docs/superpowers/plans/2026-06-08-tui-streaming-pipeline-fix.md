# TUI Streaming Pipeline Fix — P0 + P1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the streaming message display pipeline (messages not showing / showing incorrectly) and remove the unsafe pointer cast in AppController.

**Architecture:** Three coordinated fixes to the streaming data flow: (a) stop treating streaming chunks as complete messages, (b) gate streaming text visibility behind `is_streaming` flag, (c) clear streaming state on finalization. Plus one safety fix replacing raw pointer downcast with `Any::downcast_mut`.

**Tech Stack:** Rust, tokio, ratatui, crossterm, std::sync::mpsc

---

## Data Flow (Current vs Target)

### Current (broken)

```
Forwarding Thread
  agent_rx chunk → streaming_text (accumulate) + tx → rx
                                                           │
                     process_agent_response (each frame)    │
                       ├─ finalized_message? → add_message  │
                       └─ rx chunks → add_message each ←───┘  ← BUG
                           
render_messages
  ├─ messages[]  ← shows per-chunk lines (broken)
  └─ streaming_text → shows always (even after done)  ← BUG
```

**Result:** Each streaming chunk becomes a separate `[assistant model]: ...` line. After completion, streaming text duplicates the finalized message.

### Target (fixed)

```
Forwarding Thread
  agent_rx chunk → streaming_text (accumulate) only  ← rx not needed for display
                      
process_agent_response (each frame)
  ├─ finalized_message? → add_message + clear streaming + is_streaming=false
  └─ rx drained (no-op for chunks, detect Disconnected for fallback)

render_messages
  ├─ messages[]  ← only finalized messages
  └─ streaming_text → shows only when is_streaming=true
```

**Result:** Single `[assistant model]: <complete text>` line after completion. No per-chunk artifacts. Streaming text cleanly hidden when done.

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `swiftforge/src/tui/views/view.rs` | Add `Any` supertrait to `View` | ~1 |
| `swiftforge/src/tui/views/chat_view.rs` | Gate streaming text visibility + clear on completion | ~10 |
| `swiftforge/src/tui/app_controller.rs` | Fix `process_agent_response` chunk handling + replace `get_chat_view_mut` with safe downcast | ~30 |

---

### Task 1: Gate streaming text visibility behind `is_streaming`

**File:** `swiftforge/src/tui/views/chat_view.rs:138-147`

**Problem:** `render_messages` always renders `streaming_text` if it has content, even after streaming is done (`is_streaming = false`). This causes duplicate display: finalized message appears in `messages[]` AND streaming text shows below it.

**Fix:** Wrap the streaming text rendering block in `if self.state.is_streaming`.

- [ ] **Step 1: Add `is_streaming` guard**

Edit `chat_view.rs` line 138:

```rust
// BEFORE (line 138-147):
        if let Ok(streaming) = ui_state.streaming_text.lock() {
            if let Some(ref text) = *streaming {
                lines.push(Line::from(Span::styled(
                    format!("[assistant {}]: ", self.state.current_model),
                    Style::new().cyan().bold(),
                )));
                lines.push(Line::from(Span::raw(text.clone())));
                lines.push(Line::from(Span::styled("▌", Style::new().slow_blink())));
            }
        }

// AFTER:
        if self.state.is_streaming {
            if let Ok(streaming) = ui_state.streaming_text.lock() {
                if let Some(ref text) = *streaming {
                    lines.push(Line::from(Span::styled(
                        format!("[assistant {}]: ", self.state.current_model),
                        Style::new().cyan().bold(),
                    )));
                    lines.push(Line::from(Span::raw(text.clone())));
                    lines.push(Line::from(Span::styled("▌", Style::new().slow_blink())));
                }
            }
        }
```

- [ ] **Step 2: Verify with `lsp_diagnostics`**

Run diagnostics on `chat_view.rs` — expect clean.

```bash
# Manual check (or use tool):
# lsp_diagnostics on swiftforge/src/tui/views/chat_view.rs
```

---

### Task 2: Fix `process_agent_response` — stop treating chunks as messages + handle Disconnected

**File:** `swiftforge/src/tui/app_controller.rs:568-629`

**Problem:** Every streaming chunk from `rx` is added as a separate message via `add_message("assistant", &chunk)`. If streaming produces N chunks, N messages appear, each with a separate `[assistant model]:` header. Also, `TryRecvError::Disconnected` is silently ignored (the `while let Ok` pattern filters it out).

**Fix:**
1. Replace `while let Ok(result) = rx.try_recv()` with a `loop { match rx.try_recv() { ... } }` that explicitly handles all variants.
2. On `Ok(chunk)` — do NOT add to messages (forwarding thread already handles `streaming_text`).
3. On `Err(TryRecvError::Disconnected)` — detect channel closure; if no `finalized_message` arrived and we're still streaming, migrate `streaming_text` to `messages` as fallback.
4. After processing `finalized_message`, clear `streaming_text` and `response_receiver`.

- [ ] **Step 1: Rewrite `process_agent_response`**

Replace the entire method body (lines 568-629):

```rust
    fn process_agent_response(&mut self) {
        debug!("[app_controller]", "process_agent_response called");

        // Step 1: Check for finalized message from async task
        let finalized_msg = {
            if let Ok(mut finalized) = self.ui_state.finalized_message.lock() {
                finalized.take()
            } else {
                None
            }
        };

        // Step 2: Drain the channel — detect Disconnected signal
        let mut channel_disconnected = false;
        {
            if let Ok(receiver) = self.ui_state.response_receiver.lock() {
                if let Some(ref rx) = *receiver {
                    debug!("[app_controller]", "Polling rx.try_recv()");
                    loop {
                        match rx.try_recv() {
                            Ok(Ok(chunk)) => {
                                // Forwarding thread already accumulates in streaming_text.
                                // No need to add as separate message.
                                debug!(
                                    "[app_controller]",
                                    "CHUNK: received, chunk_len={} (forwarded to streaming_text)",
                                    chunk.len()
                                );
                            }
                            Ok(Err(e)) => {
                                debug!("[app_controller]", "CHUNK: agent error: {:?}", e);
                            }
                            Err(mpsc::TryRecvError::Empty) => {
                                // No more data — normal case
                                break;
                            }
                            Err(mpsc::TryRecvError::Disconnected) => {
                                // Channel closed — agent task finished sending chunks
                                debug!("[app_controller]", "CHANNEL: disconnected");
                                channel_disconnected = true;
                                break;
                            }
                        }
                    }
                } else {
                    debug!("[app_controller]", "response_receiver is None");
                }
            }
        }

        // Step 3: Apply state transitions
        if let Some(chat_view) = self.get_chat_view_mut() {
            // 3a: Finalized message from async task (preferred path)
            if let Some((role, content)) = finalized_msg {
                debug!(
                    "[app_controller]",
                    "FINALIZED: adding to messages, role={}, content_len={}",
                    role,
                    content.len()
                );
                chat_view.state.add_message(&role, &content);
                chat_view.state.is_streaming = false;
                // Clear streaming state to avoid duplicate display
                self.ui_state.clear_streaming();
                if let Ok(mut receiver) = self.ui_state.response_receiver.lock() {
                    *receiver = None;
                }
            }
            // 3b: Channel closed but no finalized_message — fallback to streaming_text
            else if channel_disconnected && chat_view.state.is_streaming {
                let streaming_text = self.ui_state.streaming_text.lock()
                    .ok()
                    .and_then(|mut s| s.take());
                if let Some(text) = streaming_text {
                    if !text.is_empty() {
                        debug!(
                            "[app_controller]",
                            "DISCONNECTED: migrating streaming_text to messages, len={}",
                            text.len()
                        );
                        chat_view.state.add_message("assistant", &text);
                        chat_view.state.is_streaming = false;
                    }
                }
                if let Ok(mut receiver) = self.ui_state.response_receiver.lock() {
                    *receiver = None;
                }
            }
        }
    }
```

- [ ] **Step 2: Add `TryRecvError` import at top of file**

`TryRecvError` is in `std::sync::mpsc` which is already imported (`use std::sync::mpsc;` on line 8). The qualified path `mpsc::TryRecvError` is used in the new code, so no additional import is needed.

- [ ] **Step 3: Verify with `lsp_diagnostics`**

Run diagnostics on `app_controller.rs`.

---

### Task 3: Replace unsafe `get_chat_view_mut` with safe `downcast_mut`

**Files:**
- `swiftforge/src/tui/views/view.rs:1-11` — add `Any` supertrait
- `swiftforge/src/tui/app_controller.rs:353-363` — replace implementation

**Problem:** `get_chat_view_mut()` uses raw pointer casting (`*mut dyn View` → `*mut ChatView`) which produces undefined behavior if `current_view` is not a `ChatView` (e.g., after switching to ConfigView).

**Fix:** Add `Any` as a supertrait of `View`, then use `Box<dyn View>::downcast_mut::<ChatView>()` which returns `Option<&mut ChatView>` safely.

- [ ] **Step 1: Add `Any` supertrait to `View` trait**

Edit `view.rs`:

```rust
// BEFORE (line 6):
pub trait View {

// AFTER:
pub trait View: std::any::Any {
```

No changes needed to `ChatView` or `ConfigView` — all concrete types already implement `Any`.

- [ ] **Step 2: Replace `get_chat_view_mut` implementation**

Edit `app_controller.rs` lines 353-363:

```rust
// BEFORE (lines 353-363):
    fn get_chat_view_mut(&mut self) -> Option<&mut ChatView> {
        let ptr = self.current_view.as_mut() as *mut dyn View;
        if ptr.is_null() {
            return None;
        }
        let chat_ptr = ptr as *mut ChatView;
        if chat_ptr.is_null() {
            return None;
        }
        unsafe { Some(&mut *chat_ptr) }
    }

// AFTER:
    fn get_chat_view_mut(&mut self) -> Option<&mut ChatView> {
        self.current_view.downcast_mut::<ChatView>()
    }
```

- [ ] **Step 3: Verify with `lsp_diagnostics`**

Run diagnostics on both `view.rs` and `app_controller.rs`.

---

### Task 4: Build and smoke test

- [ ] **Step 1: Build the project**

```bash
cd swiftforge
cargo build --bin swiftforge 2>&1
```

Expected: Clean compilation, no warnings.

- [ ] **Step 2: Run existing tests**

```bash
cargo test 2>&1
```

Expected: All tests pass. Note any pre-existing failures.

- [ ] **Step 3: Final diagnostic sweep**

```bash
# Check for any lingering issues
lsp_diagnostics on swiftforge/src/tui/
```

---

## Verification Checklist

- [ ] `chat_view.rs`: streaming text only renders when `is_streaming == true`
- [ ] `app_controller.rs`: no `add_message` per-chunk in `process_agent_response`
- [ ] `app_controller.rs`: `TryRecvError::Disconnected` triggers fallback migration
- [ ] `app_controller.rs`: `finalized_message` processing clears `streaming_text` and `response_receiver`
- [ ] `app_controller.rs`: no `unsafe` blocks remain in `get_chat_view_mut`
- [ ] `view.rs`: `View` trait has `Any` supertrait
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo test` passes

---

## Edge Cases Covered

| Case | Behavior |
|------|----------|
| Normal completion (finalized_message set) | `add_message` with complete text, streaming state cleared |
| Streaming produces chunks but empty final response | Disconnected fallback migrates `streaming_text` to messages |
| Disconnected before finalized_message | Fallback uses `streaming_text` content |
| Empty streaming (no chunks, no response) | Nothing added to messages, `is_streaming` stays true (will be handled on next message) |
| View switched mid-streaming | `get_chat_view_mut` returns `None` (safe, no UB) |
| Multiple rapid chunks | Accumulated in `streaming_text`, no per-chunk message artifacts |
