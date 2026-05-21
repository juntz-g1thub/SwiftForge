# 文档体系规范

> 版本: 1.0
> 日期: 2026-05-20
> 状态: 已批准

---

## 一、文档分级

| 级别 | 名称 | 说明 | 审批 |
|------|------|------|------|
| L1 | 规范/标准 | 代码规范、commit规范、流程规范 | explicit approve |
| L2 | 架构/接口 | 系统架构、模块接口、协议定义 | explicit approve |
| L3 | 设计/Spec | 功能设计、重构方案、技术决策 | brainstorming review (算 explicit approve) |
| L4 | 记录/分析 | Bug分析、决策记录、事后复盘 | 无 |

---

## 二、目录结构

```
docs/
├── standards/              # L1 规范/标准
│   ├── YYYY-MM-DD-coding-standards.md
│   └── YYYY-MM-DD-git-conventions.md
│
├── architecture/           # L2 架构/接口
│   ├── YYYY-MM-DD-platform-architecture.md
│   └── YYYY-MM-DD-<module>-architecture.md
│
├── specs/                  # L3 设计/Spec (brainstorming skill 输出)
│   └── YYYY-MM-DD-<feature>-design.md
│
├── records/                # L4 记录/分析
│   ├── bugs/
│   │   └── bug-YYYY-MM-DD-<issue>.md
│   ├── decisions/
│   │   └── decision-YYYY-MM-DD-<topic>.md
│   └── postmortems/
│       └── postmortem-YYYY-MM-DD-<incident>.md
```

---

## 三、命名规则

### L1 standards
```
YYYY-MM-DD-<规范名>.md
例: 2026-05-20-coding-standards.md
```

### L2 architecture
```
YYYY-MM-DD-<模块名>-architecture.md
例: 2026-05-20-platform-architecture.md
    2026-05-20-tui-module-architecture.md
```

### L3 specs
```
YYYY-MM-DD-<功能名>-design.md
例: 2026-05-20-streaming-feature-design.md
```

### L4 records
```
<类型>-YYYY-MM-DD-<描述>.md
例: bug-2026-05-19-tui-messages-not-showing.md
    decision-2026-05-18-ratatui-selection.md
    postmortem-2026-05-15-deploy-failure.md
```

---

## 四、审批流程

### L1/L2 文档
1. 编写文档
2. 提交给用户 review
3. 用户 explicit approve
4. 提示用户完成git提交

### L3 文档
- 通过 brainstorming skill 生成
- skill 自动执行 user review 环节
- approve 后直接使用，提示用户完成git提交

### L4 文档
- 无需审批
- 完成后提示用户完成git提交

---

## 五、版本策略

**所有文档随代码版本化**：

- 文档改动与对应代码在同一 commit
- 查看某 commit 时能看到当时的文档
- 主要 milestone 的文档与代码 tag 对齐

---

## 六、其他

### Session 文件
- 位置: 项目根目录 `session-*.md`
- 不纳入正式文档体系
- 不被 git 跟踪
- 作为开发过程证据保留

### Skill 路径覆盖
brainstorming 和 writing-plans skill 的默认路径被本规范覆盖：
- `docs/superpowers/specs/` → `docs/specs/`
- `docs/superpowers/plans/` → `docs/specs/`