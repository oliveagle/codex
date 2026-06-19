---
name: ole-task-check
description: "只读检查 beads-rust 任务。用 br list 列出任务并验收最近已完成任务的质量。检查质量门脚本。发现问题则 reopen 或创建新任务。禁止开发/编写代码。Trigger: /ole-task-check, check tasks, 检查任务"
---

# ole-task-check

只读检查 beads-rust 任务状态和已完成任务的质量。

## 核心原则

**禁止开发/编写任何代码**。你只做检查和验收，不做实现。

## 工作流程

### 1. 列出所有任务

```bash
br list
```

查看所有任务的状态（open/closed），包括 epic 和子任务。

### 2. 检查 open 任务

- 查看每个 open 任务的标题和描述
- 确认依赖关系：`br dep list`
- 报告当前还有哪些任务未完成、各自的阻塞情况

### 3. 验收最近已关闭的任务

对于最近关闭（`br close`）的任务：
1. 读取任务的标题和描述，理解原始要求
2. 在代码库中检查对应的实现：
   - 代码是否实现了任务描述的所有要求
   - 是否添加了测试（能写测试的功能必须有测试）
   - 是否有 placeholder/TODO/stub 实现（不允许）
   - 代码质量是否符合项目规范
3. 检查测试是否覆盖了核心功能和边界情况

### 4. 检查研发规范合规性

检查代码是否遵守项目的研发规范（CLAUDE.md / AGENTS.md）：
- 是否有 placeholder/TODO/stub 实现（禁止）
- 文件是否超过 500 行（必须拆分）
- `AGENTS.md` / `CLAUDE.md` 是否超过 500 行（超过必须拆分）
- 是否有 placeholder 实现、注释掉的代码、未使用的导入
- 代码风格是否符合项目规范
- 是否有安全隐患（如命令注入、XSS、SQL 注入等）
- 文档命名是否符合日期后缀规范（如 `plan_xxx_20260619.md`）

### 4a. 覆盖率要求

Go 项目必须满足以下覆盖率门禁：

| 指标 | 阈值 | 工具 |
|------|------|------|
| 行覆盖率 (Line Coverage) | ≥ 80% | `go tool cover` |
| 分支覆盖率 (Branch Coverage) | ≥ 80% | `gobco` |

**检查方法**：
```bash
# 行覆盖率
go test ./... -count=1 -coverprofile=coverage.out
go tool cover -func=coverage.out | grep 'total:'

# 分支覆盖率（需要 gobco-tool）
go test -cover -toolexec 'gobco-tool' ./... -count=1
```

**未达标时**：创建新任务要求补充测试：
```bash
br create --title="[覆盖率] 行/分支覆盖率低于 80%" --description="..."
```

**未安装 gobco-tool 时**：创建新任务要求安装：
```bash
br create --title="[工具] 安装 gobco 分支覆盖率工具" --description="执行: go install github.com/junhwi/gobco/...@latest"
```

**不符合规范时**：创建新任务要求修复：
```bash
br create --title="[规范违规] 简要描述" --description="..."
```

### 4b. 检查质量门脚本

**检查是否存在**：
```bash
ls .githooks/quality-gate.sh .git/hooks/pre-push 2>/dev/null
```

- **如果不存在**：将技能自带的模板（`quality-gate-template.sh`）复制到项目的 `.githooks/quality-gate.sh`，并安装到 git hook：
  ```bash
  cp <skill_dir>/quality-gate-template.sh .githooks/quality-gate.sh
  chmod +x .githooks/quality-gate.sh
  ln -sf ../../.githooks/quality-gate.sh .git/hooks/pre-push
  ```
- **如果已存在**：检查内容是否过期，必要时用模板更新
- 质量门脚本包含的检查项：编译、测试、**行覆盖率（≥80%）**、**分支覆盖率（≥80%，gobco）**、placeholder/TODO 检查、文件行数限制、CLAUDE.md/AGENTS.md 行数检查、安全扫描、文档命名规范

**缺失或未安装时**：创建新任务要求修复：
```bash
br create --title="[质量门] 添加/修复 quality-gate.sh 并安装 git hook" --description="..."
```

### 5. 发现问题时的处理

- **实现不完整或有 bug**：`br reopen <issue-id>` 重新打开任务
- **缺少测试**：`br reopen <issue-id>` 重新打开，要求补充测试
- **实现有误或偏离需求**：创建新任务描述问题
- **不符合研发规范**：创建新任务要求修复（见上一步）
- **实现正确**：标记为验收通过，无需操作

### 6. 经验教训沉淀

检查过程中发现的问题和积累的经验，必须沉淀到项目文档中：

**更新项目文档**：
- 如果发现某类问题反复出现，更新 `CLAUDE.md` 或 `AGENTS.md`，记录该规范/注意事项
- 如果发现好的实践，也记录到文档中供后续参考
- 更新格式：在相应章节追加内容，或新增一个章节/表格

**沉淀内容类型**：
| 类型 | 示例 | 写入位置 |
|------|------|----------|
| 常见问题 | "XX 模块容易忘记添加超时控制" | CLAUDE.md 注意事项 |
| 规范补充 | "新引入的 XX 库必须配置连接池" | CLAUDE.md 技术规范 |
| 好经验 | "XX 模式在该模块中表现良好" | AGENTS.md 实践经验 |
| 反模式 | "禁止在 XX 场景下使用 YY" | CLAUDE.md 禁止项 |

**注意事项**：
- 文档更新要具体、可执行，避免模糊描述
- 如果是新增章节，要在文档的目录/索引中登记
- 保持文档简洁，删除过时或重复的内容
- **如果 `AGENTS.md` 或 `CLAUDE.md` 超过 500 行，必须拆分为多个文件**：
  - 按主题/功能拆分为 `docs/` 下的独立文件（如 `docs/rules/coding-standards.md`、`docs/rules/testing-standards.md`）
  - 在 `AGENTS.md` / `CLAUDE.md` 中保留摘要和指向子文档的链接
  - 拆分时保持原有内容完整，不要删除原始内容，只做重组
  - 每次沉淀经验时也要检查：如果当前文档已过大，优先写入子文档而非追加到主文档

## 输出

检查完成后输出一份报告：
- 当前 open 任务列表及状态
- 最近关闭任务的验收结果（通过/不通过及原因）
- 研发规范合规检查结果（有无违规、新建的修复任务）
- 质量门脚本检查结果（是否存在、已安装、是否需要更新）
- 经验教训沉淀情况（更新了哪些文档、新增了什么内容）
- 需要 reopen 或新建的任务
