#!/usr/bin/env bash
# ============================================================
# 质量门脚本模板
#
# 用途: pre-commit / pre-push 质量检查
# 安装: cp .githooks/quality-gate.sh .git/hooks/pre-push && chmod +x .git/hooks/pre-push
# 或:  ln -sf ../../.githooks/quality-gate.sh .git/hooks/pre-commit
# ============================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
ERRORS=0

log_pass() { echo -e "  ${GREEN}PASS${NC}  $1"; }
log_fail() { echo -e "  ${RED}FAIL${NC}  $1"; ERRORS=$((ERRORS + 1)); }
log_warn() { echo -e "  ${YELLOW}WARN${NC}  $1"; }
log_info() { echo -e "${GREEN}==>${NC} $1"; }

# ============================================================
# 0. 项目根目录
# ============================================================
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$PROJECT_ROOT"

log_info "质量门检查开始: $(pwd)"

# ============================================================
# 1. 编译/构建检查
# ============================================================
log_info "1. 编译检查"

if command -v go &>/dev/null && [ -f go.mod ]; then
    if go build ./... 2>&1; then
        log_pass "Go 编译通过"
    else
        log_fail "Go 编译失败"
    fi
elif command -v cargo &>/dev/null && [ -f Cargo.toml ]; then
    if cargo check 2>&1; then
        log_pass "Rust 编译检查通过"
    else
        log_fail "Rust 编译检查失败"
    fi
else
    log_warn "未检测到 Go 或 Rust 项目，跳过编译检查"
fi

# ============================================================
# 2. 测试检查 + 覆盖率
# ============================================================
log_info "2. 测试检查"

if command -v go &>/dev/null && [ -f go.mod ]; then
    # 2a. 运行测试并生成覆盖率 profile
    if go test ./... -count=1 -coverprofile=coverage.out 2>&1; then
        log_pass "Go 测试通过"
    else
        log_fail "Go 测试失败"
    fi

    # 2b. 行覆盖率检查 (≥80%)
    if [ -f coverage.out ]; then
        LINE_COVER=$(go tool cover -func=coverage.out | grep 'total:' | awk '{print $3}' | tr -d '%')
        if [ -n "$LINE_COVER" ]; then
            if awk "BEGIN{exit ($LINE_COVER >= 80) ? 0 : 1}"; then
                log_pass "行覆盖率: ${LINE_COVER}% (≥80%)"
            else
                log_fail "行覆盖率: ${LINE_COVER}% (要求 ≥80%)"
            fi
        else
            log_warn "无法解析行覆盖率"
        fi
    fi

    # 2c. 分支覆盖率检查 (≥80%，使用 gobco)
    if command -v gobco-tool &>/dev/null && [ -f go.mod ]; then
        GOBCO_OUTPUT=$(go test -cover -toolexec 'gobco-tool' ./... -count=1 2>&1 || true)
        # gobco 终端输出格式: "filename covered / total" per file，最后一行是汇总
        # 汇总行格式: "total: covered / total"
        GOBCO_SUMMARY=$(echo "$GOBCO_OUTPUT" | grep '^total:' | head -1 || true)
        if [ -n "$GOBCO_SUMMARY" ]; then
            COVERED=$(echo "$GOBCO_SUMMARY" | awk -F'/' '{print $1}' | awk '{print $NF}')
            TOTAL=$(echo "$GOBCO_SUMMARY" | awk -F'/' '{print $2}' | awk '{print $1}')
            if [ -n "$COVERED" ] && [ -n "$TOTAL" ] && [ "$TOTAL" -gt 0 ] 2>/dev/null; then
                BRANCH_COVER=$(awk "BEGIN{printf \"%.1f\", ($COVERED/$TOTAL)*100}")
                if awk "BEGIN{exit ($BRANCH_COVER >= 80) ? 0 : 1}"; then
                    log_pass "分支覆盖率: ${BRANCH_COVER}% (${COVERED}/${TOTAL}, ≥80%)"
                else
                    log_fail "分支覆盖率: ${BRANCH_COVER}% (${COVERED}/${TOTAL}, 要求 ≥80%)"
                fi
            else
                log_warn "分支覆盖率: 无分支可测量"
            fi
        else
            log_warn "分支覆盖率: gobco 未输出汇总行"
        fi
    else
        log_warn "gobco-tool 未安装，跳过分支覆盖率检查 (安装: go install github.com/junhwi/gobco/...@latest)"
    fi

    # 清理临时文件
    rm -f coverage.out gobco.out gobco.html

elif command -v cargo &>/dev/null && [ -f Cargo.toml ]; then
    if cargo test 2>&1; then
        log_pass "Rust 测试通过"
    else
        log_fail "Rust 测试失败"
    fi
else
    log_warn "未检测到 Go 或 Rust 项目，跳过测试检查"
fi

# ============================================================
# 3. Placeholder/TODO 检查
# ============================================================
log_info "3. Placeholder/TODO 检查"

# 检查源代码中的 placeholder/TODO/stub 关键词
# 排除: 注释文件、文档、git 目录
PLACEHOLDER_PATTERN='(placeholder|TODO:|FIXME|stub|TBD|Not implemented|not yet implemented)'

FOUND=$(grep -rn --include='*.go' --include='*.rs' --include='*.ts' --include='*.js' \
    --include='*.py' --include='*.vue' --include='*.tsx' --include='*.jsx' \
    -i -E "$PLACEHOLDER_PATTERN" \
    --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target \
    --exclude='*.md' --exclude='*.txt' --exclude='*generated*' \
    2>/dev/null || true)

if [ -z "$FOUND" ]; then
    log_pass "未发现 placeholder/TODO/stub"
else
    log_fail "发现 placeholder/TODO/stub:\n$FOUND"
fi

# ============================================================
# 4. 文件行数检查 (500 行限制)
# ============================================================
log_info "4. 文件行数检查（所有文件）"

# 排除: .git, node_modules, target, vendor, third_party, 二进制文件, 已压缩/打包文件
OVERSIZED=$(find . -type f \
    -not -path './.git/*' \
    -not -path '*/node_modules/*' \
    -not -path '*/target/*' \
    -not -path '*/vendor/*' \
    -not -path '*/third_party/*' \
    -not -path './.beads/*' \
    -not -name '*.min.*' \
    -not -name '*.map' \
    -not -name '*.lock' \
    -not -name '*.sum' \
    -not -name '*.mod' \
    -not -name 'Cargo.lock' \
    -not -name '*.png' -not -name '*.jpg' -not -name '*.jpeg' \
    -not -name '*.gif' -not -name '*.svg' -not -name '*.ico' \
    -not -name '*.woff' -not -name '*.woff2' -not -name '*.ttf' \
    -not -name '*.wasm' -not -name '*.bin' -not -name '*.exe' \
    -not -name '*.so' -not -name '*.dll' -not -name '*.dylib' \
    -not -name '*.db' -not -name '*.sqlite' -not -name '*.sqlite3' \
    -not -name '*.tar' -not -name '*.gz' -not -name '*.zip' \
    -exec awk 'END{if(NR>500) print FILENAME": "NR" lines"}' {} + 2>/dev/null || true)

if [ -z "$OVERSIZED" ]; then
    log_pass "所有文件未超过 500 行"
else
    log_fail "以下文件超过 500 行（需拆分）:\n$OVERSIZED"
fi

# ============================================================
# 5. CLAUDE.md / AGENTS.md 行数检查
# ============================================================
log_info "5. CLAUDE.md / AGENTS.md 行数检查"

for f in CLAUDE.md AGENTS.md; do
    if [ -f "$f" ]; then
        # 如果是软链接，跟踪到实际文件
        LINES=$(wc -l < "$(readlink -f "$f" 2>/dev/null || echo "$f")" 2>/dev/null || echo 0)
        if [ "$LINES" -gt 500 ]; then
            log_fail "$f 超过 500 行 (${LINES} 行)，需要拆分为 docs/ 下的子文档"
        else
            log_pass "$f (${LINES} 行)"
        fi
    fi
done

# ============================================================
# 6. 安全扫描
# ============================================================
log_info "6. 安全检查"

# 检查常见安全问题
SECURITY_ISSUES=""

# 硬编码密钥/密码模式
SECRETS=$(grep -rn --include='*.go' --include='*.rs' --include='*.ts' --include='*.py' \
    -i -E '(password|secret|api_key|token|auth)\s*=\s*"[^"]{8,}"' \
    --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target \
    --exclude='*_test*' --exclude='*_spec*' \
    2>/dev/null || true)
if [ -n "$SECRETS" ]; then
    SECURITY_ISSUES="${SECURITY_ISSUES}\n[硬编码密钥/密码]:\n${SECRETS}\n"
fi

# SQL 拼接模式（Go）
SQL_INJECT=$(grep -rn --include='*.go' -E '(fmt\.Sprintf|strings\.Join).*("SELECT|INSERT|UPDATE|DELETE)' \
    --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target \
    2>/dev/null || true)
if [ -n "$SQL_INJECT" ]; then
    SECURITY_ISSUES="${SECURITY_ISSUES}\n[潜在 SQL 注入]:\n${SQL_INJECT}\n"
fi

if [ -z "$SECURITY_ISSUES" ]; then
    log_pass "未发现明显安全问题"
else
    log_fail "发现潜在安全问题:${SECURITY_ISSUES}"
fi

# ============================================================
# 7. 文档命名规范检查
# ============================================================
log_info "7. 文档命名规范检查"

# 检查 docs/development/ 下的文档是否符合日期后缀规范
# 有时间相关性的文档应该使用 _YYYYMMDD.md 后缀
BAD_NAMES=""
for dir in docs/development docs/research docs/records; do
    if [ -d "$dir" ]; then
        # 计划/设计/PRD 等文档应该有日期后缀
        while IFS= read -r -d '' file; do
            BASENAME=$(basename "$file")
            # 排除已经带日期后缀的文件
            if [[ "$BASENAME" =~ ^(plan|design|prd|todo|research|meeting|adr|implementation|test_plan|release|review)_.*\.md$ ]] && \
               ! [[ "$BASENAME" =~ _[0-9]{8}\.md$ ]]; then
                BAD_NAMES="${BAD_NAMES}\n  - $file"
            fi
        done < <(find "$dir" -maxdepth 1 -name '*.md' -print0 2>/dev/null)
    fi
done

if [ -z "$BAD_NAMES" ]; then
    log_pass "文档命名符合规范"
else
    log_warn "以下文档可能需要日期后缀:${BAD_NAMES}"
fi

# ============================================================
# 总结
# ============================================================
echo ""
if [ "$ERRORS" -gt 0 ]; then
    echo -e "${RED}质量门检查失败: $ERRORS 项检查未通过${NC}"
    exit 1
else
    echo -e "${GREEN}质量门检查全部通过${NC}"
    exit 0
fi
