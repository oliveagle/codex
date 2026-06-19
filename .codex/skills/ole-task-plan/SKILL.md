---
name: ole-task-plan
description: "不开发代码，只通过 br create 创建 EPIC/UserStory/Task 任务树，保证依赖关系正确、有验收标准。Trigger: /ole-task-plan, plan tasks, 任务规划"
---

# ole-task-plan

不开发代码，只通过 `br` CLI 创建结构化的任务树。

## 核心原则

**禁止开发/编写任何代码**。你只做任务规划和创建，不做实现。

## 工作流程

### 1. 分析用户需求

仔细分析用户给出的任务描述，理解：
- 目标是什么
- 涉及哪些模块/文件
- 需要哪些步骤
- 各步骤之间的先后依赖关系

### 2. 拆分任务

将任务拆分为三层结构：

| 层级 | 类型 | 说明 |
|------|------|------|
| **EPIC** | `--type=epic` | 整体功能/特性，作为父任务 |
| **User Story** | 默认类型 | 用户故事，描述从用户角度可完成的功能点 |
| **Task** | 默认类型 | 具体实现任务，可独立验证完成 |

### 3. 创建任务

```bash
# 创建 EPIC
br create --type=epic \
  --title="[EPIC] 功能名称" \
  --description="$(cat <<'EOF'
[功能描述，包含背景和目标]
EOF
)"

# 创建 User Story
br create --parent=<epic-id> \
  --title="US-XXX: 用户故事标题" \
  --description="$(cat <<'EOF'
As a [角色], I want [目标], so that [价值].

## 验收标准
- [ ] 具体可验证的条件1
- [ ] 具体可验证的条件2
EOF
)" \
  --priority=1

# 创建 Task
br create --parent=<epic-id> \
  --title="TASK: 任务描述" \
  --description="$(cat <<'EOF'
[任务详细说明]

## 验收标准
- [ ] 具体可验证的条件
EOF
)" \
  --priority=2
```

### 4. 设置依赖关系

```bash
# B 依赖 A（B 必须在 A 完成后才能开始）
br dep add <B-id> <A-id>
```

依赖规则：
- User Story 之间如有依赖，按依赖顺序设置
- Task 如果依赖某个 User Story，设置对应依赖
- 不允许循环依赖

### 5. 验收标准

**每个 User Story 和 Task 都必须有验收标准**，写在描述的 `## 验收标准` 部分。

验收标准要求：
- **具体可验证**：能通过代码检查、测试运行、功能验证来确认
- **不要模糊描述**：如"工作正常"、"性能良好"等不可接受
- **覆盖核心功能**：主要功能路径必须覆盖
- **覆盖边界情况**：关键边界条件要考虑

好的示例：
```
## 验收标准
- [ ] 添加 session_timeout 字段到 Config 结构体
- [ ] session_timeout 默认值为 30 分钟
- [ ] 超过 timeout 后 session affinity 缓存失效
- [ ] 单元测试覆盖超时逻辑
- [ ] go build 和 go test 通过
```

坏的示例：
```
## 验收标准
- [ ] 超时功能正常工作  # 太模糊
- [ ] 性能没有明显下降  # 不可衡量
```

## 注意事项

- 使用 `<<'EOF'`（单引号）HEREDOC，防止 shell 解释特殊字符
- 每个任务的描述必须清晰完整，包含上下文和验收标准
- 优先创建依赖少的基础任务
- 如果任务范围不明确，优先拆分到可在一次迭代内完成的粒度
- 任务创建完成后输出完整任务树概览（ID、标题、依赖关系）
