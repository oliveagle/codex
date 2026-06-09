#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATCHES_DIR="$(dirname "$SCRIPT_DIR")/patches"

echo "=== ole-fix 补丁验证脚本 ==="
echo "检查补丁是否可干净应用到当前分支..."
echo ""

# 获取当前 HEAD
CURRENT_HEAD=$(git rev-parse --short HEAD)
echo "当前 HEAD: $CURRENT_HEAD"
echo ""

# 尝试应用所有补丁 (dry-run)
cd "$(git rev-parse --show-toplevel)"

FAILED=0
for patch in "$PATCHES_DIR"/*.patch; do
    patch_name=$(basename "$patch")
    echo "检查: $patch_name"
    
    if patch -p1 --dry-run < "$patch" >/dev/null 2>&1; then
        echo "✓ 可干净应用"
    else
        echo "✗ 可能冲突"
        FAILED=1
    fi
    echo ""
done

if [ $FAILED -eq 0 ]; then
    echo "=== 所有补丁可干净应用 ==="
    exit 0
else
    echo "=== 发现冲突 ==="
    echo "建议:"
    echo "1. 检查上游更改的文件"
    echo "2. 使用 git apply -3way <patch> 尝试三方合并"
    echo "3. 手动解决冲突后重新验证"
    exit 1
fi
