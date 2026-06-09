#!/bin/bash
# 测试: patches 应用到最新 main 后 cargo check 是否通过
# 注意: 这个测试会消耗时间编译. 仅在需要时运行.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== 测试: Cargo 编译检查 ==="
echo ""

# 先确保补丁可应用
"$SCRIPT_DIR/test_rebase.sh" "$@"

echo ""
echo "运行 cargo check..."
cd codex-rs
cargo check 2>&1 || {
    echo "⚠ cargo check 失败,检查编译错误"
    exit 1
}

echo "✓ 编译通过"
