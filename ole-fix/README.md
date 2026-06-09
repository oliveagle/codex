# ole-fix

用户自定义补丁集。这些补丁来自 oliveagle 的本地开发，不应与上游 codex 冲突。

## 目标

当上游 codex 有新版本时，这些补丁应当能够干净地 rebase 到最新 upstream main 上。

## 目录结构

```
ole-fix/
├── patches/                  # 原始补丁 (按 commit 顺序, git format-patch 输出)
│   ├── 0001-chore-update-deps-and-add-codegen-tests.patch
│   ├── 0002-feat-responses-api-proxy-add-Chat-Completions-API-co.patch
│   ├── 0003-feat-responses-api-proxy-add-Chat-Completions-API-su.patch
│   └── 0004-chore-remove-unused-ImagesClient-re-export.patch
├── modules/                  # 按功能拆分的补丁 (选择性重放)
│   ├── codegen-tests/        # 代码生成测试基础设施
│   ├── chat-conversion/      # Chat↔Responses 类型转换
│   ├── chat-api/             # Chat Completions API 端点 + SSE
│   ├── deps-tweaks/          # 依赖/模型/锁更新
│   └── cleanup/              # 清理 (移除未使用的 re-export)
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

### 0002 - add Chat Completions API compatibility
- 添加 /v1/chat/completions 端点转换层
- 实现 Chat↔Responses 类型定义和转换逻辑
- 更新模型信息 (models.json)
- 影响文件: `conversion/`, `types/chat.rs`, `tests/chat_completions.rs`

### 0003 - add Chat Completions API support
- 添加 Chat endpoint, request 和 SSE 类型
- 添加 Chat Completions 请求/响应处理
- 更新模型提供者信息
- 影响文件: `endpoint/chat.rs`, `requests/chat.rs`, `sse/chat.rs`

### 0004 - remove unused ImagesClient re-export
- 清理 `lib.rs` 中未使用的 `ImagesClient` 导出

## 测试验证

```bash
# 快速测试: 在 7d47056ea 上验证所有补丁可干净应用
cd codex && git checkout 7d47056ea
cd .. && ./ole-fix/tests/test_rebase.sh 7d47056ea
```
