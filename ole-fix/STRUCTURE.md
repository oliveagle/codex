# ole-fix 目录结构

```
ole-fix/
├── patches/                   # 原始 patch 文件 (按 commit 顺序)
│   ├── 0001-*.patch
│   ├── 0002-*.patch
│   ├── 0003-*.patch
│   └── 0004-*.patch
├── modules/                   # 按功能模块拆分的补丁 (独立可重放)
│   ├── chat-api/              # Chat Completions API 相关
│   │   ├── chat-types.patch
│   │   ├── chat-conversion.patch
│   │   └── chat-sse.patch
│   ├── models-info/           # 模型信息更新
│   │   └── models.json.patch
│   └── deps-tweaks/           # 依赖小改动
│       └── cargo-deps.patch
├── scripts/
│   ├── apply.sh               # 应用补丁脚本
│   ├── verify.sh              # 验证补丁与上游兼容性
│   └── rebase-helper.sh       # rebase 辅助脚本
├── tests/
│   ├── test_chat_api.sh       # 测试 Chat API 功能
│   ├── test_integration.sh     # 集成测试
│   └── test_rebase.sh         # 测试 rebase 后补丁重放
├── README.md                  # 本文档
└── STRUCTURE.md               # 结构说明

设计原则:
1. patches/ 保留原始 commit 顺序,用于回溯
2. modules/ 按功能拆分,便于选择性应用和冲突解决
3. scripts/ 自动化补丁应用和验证
4. tests/ 确保 rebase 后功能正常
