#!/bin/bash
# 测试: 在指定起点上重新应用补丁,验证干净合并
# 用法: ./test_rebase.sh [base-ref]
#       默认 base-ref = origin/main, 也可指定 commit hash

set -e

BASE_REF="${1:-origin/main}"
REPO_ROOT=$(git rev-parse --show-toplevel)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATCHES_DIR="$(dirname "$SCRIPT_DIR")/patches"
WORK_BRANCH="ole-fix-test-rebase-$$"

PASS=0
FAIL=0

cleanup() {
    git checkout --quiet "$ORIG_BRANCH" 2>/dev/null || true
    git branch -D "$WORK_BRANCH" 2>/dev/null || true
}

echo "=== 测试: Rebase 后补丁重放 ==="
echo "基准: $BASE_REF"
echo ""

ORIG_BRANCH=$(git branch --show-current)
trap cleanup EXIT

# 创建测试分支
git fetch origin 2>/dev/null || true
git checkout -b "$WORK_BRANCH" "$BASE_REF"
BASE_COMMIT=$(git rev-parse --short HEAD)
echo "测试起点: $BASE_COMMIT"
echo ""

# 尝试应用每一个补丁
echo "--- 按顺序应用补丁 ---"
for patch in "$PATCHES_DIR"/*.patch; do
    patch_name=$(basename "$patch")
    
    if git apply --3way "$patch" 2>/dev/null; then
        git add -A
        SUBJECT=$(head -1 "$patch" | sed 's/^From: //' | head -c 80)
        git commit -m "apply: $(echo "$patch_name" | sed 's/\.patch$//')" --allow-empty
        echo "  ✓ $patch_name"
        PASS=$((PASS + 1))
    else
        echo "  ✗ $patch_name (冲突)"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "=== 结果: $PASS 成功, $FAIL 失败 ==="

if [ $FAIL -gt 0 ]; then
    echo "⚠ 有补丁冲突,需要手动解决"
    exit 1
fi

echo "✓ 所有补丁可干净重放"
exit 0
