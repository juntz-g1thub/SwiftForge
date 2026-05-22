# AGENTS.md

## Project

**rust-agent-platform** — Rust TUI agent platform with streaming LLM support and tool calling.
**Source**: `rust-agent-platform/` (workspace member)

## Git Workflow

详细规范见 [.git-workflow.md](.git-workflow.md)。

> ⚠️ 所有 git 操作必须由用户手动执行，切勿自动运行 git 命令。

## Documentation System

**文档根目录**: `docs/`
**文档规范**: `docs/README.md`

### 目录结构

```
docs/
├── README.md              # 文档体系说明
├── standards/             # L1 规范/标准
├── architecture/          # L2 架构/接口
├── specs/                 # L3 设计/Spec
└── records/               # L4 记录/分析
```

### 关键规则

- **L1/L2 文档**: 需要用户 explicit approve
- **L3 文档**: 通过 brainstorming skill 生成
- **L4 文档**: 无需审批

> ⚠️ **所有工作请在 worktree 中进行，所有文档请在 worktree 的 `docs/` 中查阅**
> 除非用户明确提示直接查阅 main 分支的文件，否则一律在 worktree 中操作文件

## Current Focus

**Branch:** `feat/tui-refactor`
**Worktree:** `.worktrees/feat-tui-refactor/`

项目架构重构，尤其是 TUI 的重构。重构后的 TUI 采用 MVC 模式：
- `tui/app_controller.rs` — 主控制器
- `tui/components/` — UI 组件（input_area, message_list, scroll_bar, status_bar）
- `tui/state/` — 状态管理（action, app_context, view_state）
- `tui/views/` — 视图层（chat_view, config_view, debug_view）

## Dev Notes

- **No CI/CD**: No GitHub Actions, no pre-commit hooks found
- **No custom linting**: Uses default `cargo clippy` / `cargo fmt`
- **Logging**: `tracing` + `tracing-subscriber` → stderr (no log files unless `--debug`)
- **Session files**: `session*.md` in root are ignored by `.gitignore`

## Recommended Skills

When working on this project, use these skills to ensure consistent quality:

### Rust Development
| Skill | When to Use |
|-------|-------------|
| `rust` | Any Rust code writing - safety, performance, async patterns |
| `rust-systems-programming` | Complex concurrency, async runtime, memory management |

### Workflow
| Skill | When to Use |
|-------|-------------|
| `brainstorming` | **Before any creative work** - adding features, components, modifying behavior |
| `using-git-worktrees` | Before starting feature work - creates isolated workspace |
| `writing-plans` | Before multi-step implementation tasks |
| `subagent-driven-development` | Executing plans with independent parallel tasks |
| `verification-before-completion` | **Before commits or PRs** - run verification commands |

### Debugging & Testing
| Skill | When to Use |
|-------|-------------|
| `systematic-debugging` | **When encountering bugs** - follow structured debugging |
| `test-driven-development` | Before writing implementation code |
| `code-refactoring-refactor-clean` | When refactoring for Clean Code / SOLID principles |
| `refactor` | General refactoring tasks |

### Code Quality
| Skill | When to Use |
|-------|-------------|
| `receiving-code-review` | When receiving code review feedback |
| `requesting-code-review` | Before merging or completing features |

### Documentation
| Skill | When to Use |
|-------|-------------|
| `readme-writer` | When writing or revising README files |