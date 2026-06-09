#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULES_DIR="$(dirname "$SCRIPT_DIR")/modules"

echo "=== ole-fix 补丁应用脚本 ==="
echo "模块目录: $MODULES_DIR"
echo ""

# 按顺序应用模块
MODULES=(
    "deps-tweaks/deps-and-models.patch"
    "chat-conversion/chat-conversion.patch"
    "chat-api/chat-api.patch"
    "cleanup/cleanup.patch"
)

for module in "${MODULES[@]}"; do
    patch_file="$MODULES_DIR/$module"
    if [ -f "$patch_file" ]; then
        echo "应用: $module"
        if patch -p1 --dry-run < "$patch_file" >/dev/null 2>&1; then
            patch -p1 < "$patch_file"
            echo "✓ 成功"
        else
            echo "✗ 失败或冲突"
            echo "提示: 使用 git am --3way 处理冲突"
            exit 1
        fi
    else
        echo "⚠ 跳过 (文件不存在): $module"
    fi
    echo ""
done

echo "=== 所有补丁应用完成 ==="
echo "请运行: cd ole-fix/tests && ./test_rebase.sh"
