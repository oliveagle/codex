# ole-fix

保留 codex 对 **Chat Completions API** 的本地补丁集。

## 目标

大多数 LLM Provider (Azure OpenAI, OpenRouter, 各种自建兼容服务) 都只支持 `/v1/chat/completions` 接口模式。  
而 upstream codex 主要基于 Responses API 设计, 不直接提供 Chat Completions 兼容层。

**ole-fix 的存在意义：确保 codex 始终支持 Chat Completions API。**

### 具体要求

1. **保留 Chat Completions 支持**：每次 rebase 到上游最新版本时，必须确保 `/v1/chat/completions` 端点、转换逻辑、SSE 流式响应等功能正常工作
2. **补丁干净应用**：尽可能使补丁可干净应用到上游最新版本。如果上游变更导致补丁冲突，需要修改补丁内容以适应新的代码结构
3. **附带修复**：保留相关的修复和优化（依赖更新、清理未使用代码、codegen 测试等）
4. **优先保证功能**：如果新版本完全无法干净应用 patch，需要手动修改代码以确保 Chat Completions 支持仍然存在

### 核心功能

- `/v1/chat/completions` 端点兼容
- Chat↔Responses 类型双向转换
- 流式响应 (SSE) 支持
- Tool calls 支持
- 模型信息更新

## 目录结构

```
ole-fix/
├── patches/                  # 原始补丁 (按 commit 顺序, git format-patch 输出)
│   ├── 0001-chore-update-deps-and-add-codegen-tests.patch
│   ├── 0004-chore-remove-unused-ImagesClient-re-export.patch
│   ├── 0005-docs-clarify-ole-fix-purpose.patch
│   └── 0006-fix-codex-client-honor-Retry-After-header-for-429-re.patch
├── modules/                  # 按功能拆分的补丁 (选择性重放)
│   ├── codegen-tests/        # 代码生成测试基础设施
│   ├── chat-conversion/      # Chat↔Responses 类型转换
│   ├── chat-api/             # Chat Completions API 端点 + SSE
│   ├── deps-tweaks/          # 依赖/模型/锁更新
│   ├── cleanup/              # 清理 (移除未使用的 re-export)
│   └── rate-limit-backoff/   # 429 Retry-After 处理
├── scripts/
│   ├── apply.sh              # 按顺序应用补丁
│   ├── verify.sh             # 验证补丁是否干净 (dry-run)
│   └── rebase-helper.sh      # rebase 辅助 (拉取上游、验证、应用)
└── tests/
    ├── test_rebase.sh        # 测试补丁在指定 base 上的可应用性
    └── test_cargo_compile.sh # 编译检查
```

## 工作流

### 日常使用

```bash
# 检查补丁是否仍兼容
./ole-fix/scripts/verify.sh

# 在指定 base 上测试补丁可应用性
./ole-fix/tests/test_rebase.sh origin/main
```

### Rebase 到上游最新版本

```bash
# 1. 拉取上游最新
git fetch origin main

# 2. 运行 rebase 辅助脚本
./ole-fix/scripts/rebase-helper.sh

# 3. 如有冲突,手动解决

# 4. 测试
./ole-fix/tests/test_rebase.sh
```

### 添加新补丁

```bash
# 创建新 commit 后,导出补丁
git format-patch -o ole-fix/patches/ <base-commit>..<head-commit>

# 重新测试所有补丁兼容性
./ole-fix/tests/test_rebase.sh <base-commit>
```

### 按模块选择性应用

```bash
# 只应用 Chat API 相关补丁
git apply ole-fix/modules/chat-api/chat-api.patch

# 只应用类型转换补丁
git apply ole-fix/modules/chat-conversion/chat-conversion.patch
```

## 补丁说明

### 0001 - update deps and add codegen tests
- 更新 Cargo.lock, Cargo.toml
- 添加 codegen-tests 测试基础设施
- 影响文件: `.gitmodules`, `Cargo.*`, `codegen-tests/`

### 0004 - remove unused ImagesClient re-export
- 清理 `lib.rs` 中未使用的 `ImagesClient` 导出

### 0005 - clarify ole-fix purpose
- README 文档更新, 说明 Chat Completions API 支持目标

### 0006 - fix 429 Retry-After backoff (重要)
- 修复 codex-client 中 429 错误未处理 Retry-After 头的问题
- 支持多种速率限制头部: Retry-After, X-RateLimit-Reset-After, X-RateLimit-Reset
- 添加 MAX_RETRY_AFTER 上限 (60s), 防止过长等待
- 添加 httpdate 依赖和 7 个单元测试
- 影响文件: `codex-client/src/retry.rs`, `codex-client/Cargo.toml`

## 上游冲突处理流程

当上游 codex 更新导致补丁无法干净应用时：

### 1. 分析冲突原因

```bash
# 检查具体哪些文件冲突
git apply --reject ole-fix/patches/0002-*.patch
find . -name "*.rej" | head
```

### 2. 评估上游变更

查看上游是否：
- 移动/重命名了相关文件 → 更新补丁中的路径
- 重构了类型定义 → 适配新的类型结构
- 修改了 SSE/流式处理逻辑 → 调整转换代码
- 删除了相关 API → 寻找替代实现方式

### 3. 修改补丁

```bash
# 1. 应用除冲突补丁外的所有补丁
git apply ole-fix/patches/0001-*.patch
git apply ole-fix/patches/0003-*.patch
git apply ole-fix/patches/0004-*.patch

# 2. 手动应用冲突的补丁 (例如 0002)
# 编辑代码以适配上游最新结构

# 3. 生成新补丁
git add -A
git diff --cached > ole-fix/patches/0002-feat-chat-completions-v2.patch
```

### 4. 重新测试

```bash
./ole-fix/tests/test_rebase.sh origin/main
```

### 5. 确保功能完整

```bash
# 编译检查
cd codex-rs && cargo check

# 运行测试
cargo test --package codex-api
```

**关键原则**：即使上游重写了相关代码，也要确保 Chat Completions API 支持仍然存在。如果 upstream 完全移除了相关接口，需要在新代码库上重新实现。

## 测试验证

```bash
# 快速测试: 在 7d47056ea 上验证所有补丁可干净应用
cd codex && git checkout 7d47056ea
cd .. && ./ole-fix/tests/test_rebase.sh 7d47056ea

# 验证 Chat Completions 转换逻辑
cd codex-rs && cargo test --package codex-api conversion
```
