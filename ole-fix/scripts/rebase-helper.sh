#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OLE_FIX_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== ole-fix Rebase 辅助脚本 ==="
echo ""

# 检查是否在 main 分支
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "⚠ 当前不在 main 分支: $CURRENT_BRANCH"
    echo "建议先切换到 main: git checkout main"
    echo ""
fi

# 1. 更新上游 main
echo "步骤 1: 更新上游 main"
echo "git fetch origin main"
git fetch origin main
echo ""

# 2. 检查上游差异
echo "步骤 2: 检查上游差异"
UPSTREAM_HEAD=$(git rev-parse origin/main)
echo "上游 main: $UPSTREAM_HEAD"
git log --oneline HEAD..origin/main | head -10 || echo "(无新提交)"
echo ""

# 3. 验证补丁兼容性
echo "步骤 3: 验证补丁兼容性"
"$OLE_FIX_DIR/scripts/verify.sh" || {
    echo ""
    echo "⚠ 补丁验证失败,可能需要手动调整"
    echo ""
}
echo ""

# 4. 应用 ole-fix 补丁 (创建工作分支)
echo "步骤 4: 创建工作分支并应用补丁"
WORK_BRANCH="ole-fix-work-$(date +%Y%m%d)"
echo "git checkout -b $WORK_BRANCH origin/main"
git checkout -b "$WORK_BRANCH" origin/main 2>/dev/null || git checkout "$WORK_BRANCH"
echo ""

echo "应用 ole-fix 补丁..."
cd "$OLE_FIX_DIR" && "$SCRIPT_DIR/apply.sh" || {
    echo "⚠ 补丁应用失败,请手动解决"
    exit 1
}
echo ""

echo "=== Rebase 准备完成 ==="
echo "工作分支: $WORK_BRANCH"
echo "下一步:"
echo "1. 运行测试: cd ole-fix/tests && ./test_rebase.sh"
echo "2. 如有问题,手动修复冲突"
echo "3. 合并到 ole-fix 分支: git checkout ole-fix && git merge $WORK_BRANCH"
