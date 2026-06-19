---
name: ole-task-do
description: "用 br list 检查 beads-rust 任务列表，然后逐个解决所有 open 任务，完成后用 br close 关闭。Trigger: /ole-task-do, do tasks, 解决所有任务, 处理任务"
---

# ole-task-do

用 `br` CLI 检查并解决所有 open beads 任务。

## 工作流程

### 1. 检查任务列表

```bash
br list
```

找出所有 `status=open` 的任务（不包括 epic 类型的任务）。

### 2. 按依赖顺序处理

- 先看任务的依赖关系：`br dep list`
- 没有 blocker 的任务优先处理
- 如果有依赖，先解决依赖的任务

### 3. 开团队处理

如果任务较多、复杂度较高或涉及多个模块，**使用 TeamCreate 创建团队并分配任务给多个 Agent 并行处理**：

```
# 创建团队
TeamCreate({ team_name: "任务执行团队", description: "处理 open beads 任务" })

# 创建任务列表
TaskCreate 每个 open bead 对应一个 task

# 分配任务给团队成员
TaskUpdate({ owner: "成员名" })
```

- 简单独立任务：单 Agent 顺序处理即可
- 复杂/多模块任务：开团队，按模块/功能拆分给不同 Agent 并行
- 有依赖关系的任务：先完成前置任务再处理依赖任务

### 4. 逐个解决任务

对于每个 open 任务：
1. 读取任务的标题和描述，理解需要做什么
2. 在代码库中实现任务要求的功能
3. **添加测试**：每个 user story 只要能写测试，就应该添加对应的测试（单元测试、集成测试等），测试应覆盖核心功能和边界情况
4. 验证实现正确（编译通过、测试通过）
5. 用 `br close <issue-id>` 关闭已完成的任务

### 5. 关闭 Epic

当所有子任务都完成后，关闭对应的 epic：

```bash
br close <epic-id>
```

## 注意事项

- 每次关闭一个任务后，重新运行 `br list` 确认状态
- 如果任务描述不清楚，先尝试理解代码上下文再实现
- 遵循项目的代码规范（参考 CLAUDE.md）
- 禁止 placeholder/TODO/stub 实现
- 如果某个任务依赖尚未完成的任务，先解决依赖
- 每次实现完一个任务都要验证（如 `go build`、`cargo build` 等）
- **测试是必须的**：如果某个功能可以写测试但没有添加，该任务不算完成
