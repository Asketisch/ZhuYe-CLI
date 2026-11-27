#!/usr/bin/env bash
# tests/live/cli_e2e.sh —— TUI 真模型端到端演练套件(本地专用)。
#
# ## 定位:本地专用,不进 CI
#
# **只用于本地人工触发,不进 CI、不接 run_all.sh 门禁链**:真模型场景
# 依赖本机可达的端点,CI 环境没有。默认跳过(ZHUYE_LIVE=1)是双保险。
# 与同目录的 live/run.sh(Kilo 免费车道 × pytest 用例)互补:这里参考
# ZhuYe 主仓 `tests/live/cli_e2e.sh` 的**场景化演练**形态 —— 每个场景
# 一组结构化检查项、一个独立进程,覆盖 TUI 的端到端全链路:
#
#   真实任务(plan 模式出 plan / 批准执行 / 产物落盘 / git 提交)
#   单轮问答(回合收尾 / assistant 消息落盘 / token 用量)
#   工具调用(审批放行 / 工具成功行 / 输出可见)
#   会话续作(--resume 回放重建 scrollback / 续作再落盘)
#
# 与 ZhuYe 主仓套件的差异:那边驱动 headless `zhuye serve`(JSONL wire,
# 断言协议事件形状);这边驱动**终端 TUI**(pty + pyte 屏幕快照),断言
# 口径落在本仓的「屏幕可见文本 / 落盘文件内容 / 进程退出码」。
#
# ## 两种跑法
#
#   # 1) 自检模式(mock provider,离线、不发网络请求):验证场景脚本本身
#   ZHUYE_E2E_MOCK=1 bash tests/live/cli_e2e.sh
#
#   # 2) 真模型模式(需要配置 toml 里的端点可达)
#   ZHUYE_LIVE=1 bash tests/live/cli_e2e.sh
#
# 常用环境变量:
#   ZHUYE_E2E_TOML      —— 模型配置路径。默认依次找:本目录 cli_e2e.toml
#                          → ZhuYe 主仓 tests/live/cli_e2e.toml(端点 /
#                          模型 / 任务与验收期望都从这份配置读)
#   ZHUYE_E2E_BIN       —— zhuye-cli 二进制(默认 release 产物,缺失则构建)
#   ZHUYE_E2E_SCENARIOS —— 只跑指定场景(空格分隔,默认全部)
#   ZHUYE_E2E_KEEP=1    —— 保留临时目录(屏幕快照 / 运行日志)供翻查
#   ZHUYE_E2E_TIMEOUT   —— 单场景超时秒数(默认 1500;xhigh 思考留余量)
#
# ## 断言口径
#
# 每个场景是一组**结构化检查项**(屏幕结构标记 / 落盘产物存在性 /
# 终态语义),由 cli_e2e_run.py 逐项产出。不断言模型自由文本内容
# (模型自由发挥,文本断言不可靠);「模型没按预期走」判 skip 而非 fail。
#
# ## 隔离
#
# 每个场景一个独立临时目录:假 HOME(+ ZHUYE_HOME)+ 独立引擎配置 +
# 独立工作区 —— 会话 rollout / traces 全落临时目录,不读、不写真实
# ~/.zhuye。git 身份经假 HOME 的 .gitconfig 提供。

set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/../lib.sh"

# ── 0. 二进制 ──────────────────────────────────────────────────────────────
# 与 run_all.sh 各入口同口径:ensure_release_binary 保证产物与源码一致
# (外部 ZHUYE_CLI_BIN 则信任用户自带,不代跑 cargo)。
ensure_release_binary
BIN="$ZHUYE_CLI_BIN"
[ -x "$BIN" ] || { tcase "zhuye-cli 二进制可执行" false; tfinish; exit 1; }

# ── 1. 配置与模式 ──────────────────────────────────────────────────────────
MOCK="${ZHUYE_E2E_MOCK:-0}"
TIMEOUT="${ZHUYE_E2E_TIMEOUT:-1500}"
KEEP="${ZHUYE_E2E_KEEP:-0}"

if [ "$MOCK" != "1" ] && [ "${ZHUYE_LIVE:-}" != "1" ]; then
    echo "  [SKIP] TUI 端到端演练套件 —— 未设 ZHUYE_LIVE=1(真模型)且未设 ZHUYE_E2E_MOCK=1(自检),默认不发网络请求"
    exit 0
fi

if ! require_cmd python3; then
    echo "  [SKIP] 未找到 python3;演练套件跳过"
    exit 0
fi
if ! python3 -c "import pyte" 2>/dev/null; then
    echo "  [SKIP] 未安装 pyte(pip install pyte);演练套件跳过"
    exit 0
fi

# 配置解析顺序:显式 env → 本目录 cli_e2e.toml → ZhuYe 主仓同款配置。
# 主仓那份带着端点 / 模型 / 任务与验收期望([task] 段),真模型模式的
# 任务场景直接消费它;本目录若放了本地副本则本地优先。
TOML="${ZHUYE_E2E_TOML:-}"
if [ -z "$TOML" ] && [ -f "$HERE/cli_e2e.toml" ]; then
    TOML="$HERE/cli_e2e.toml"
fi
if [ -z "$TOML" ] && [ -f "$ROOT/../ZhuYe/tests/live/cli_e2e.toml" ]; then
    TOML="$ROOT/../ZhuYe/tests/live/cli_e2e.toml"
fi
if [ "$MOCK" != "1" ] && [ -z "$TOML" ]; then
    echo "  [SKIP] TUI 端到端演练套件 —— 找不到模型配置(设 ZHUYE_E2E_TOML 指向 cli_e2e.toml)"
    exit 0
fi
# mock 自检不需要端点;给 runner 一个存在的占位路径,场景脚本里的
# toml 读取才不会因缺文件炸掉(字段全走默认值)。
if [ "$MOCK" = "1" ] && [ -z "$TOML" ]; then
    TOML="$(mktemp "${TMPDIR:-/tmp}/zhuye-cli-e2e-selfcheck.XXXXXX")"
    printf '[endpoint]\nbase_url = ""\napi_key = ""\nmodel = ""\n' > "$TOML"
fi

# ── 2. 场景清单(来自注册表;可用 ZHUYE_E2E_SCENARIOS 过滤)──────────────
SCENARIOS=$(python3 - "$HERE" << 'PYEOF'
import sys
sys.path.insert(0, sys.argv[1])
from cli_e2e_scenarios import SCENARIOS
print(" ".join(sorted(SCENARIOS)))
PYEOF
)
if [ -n "${ZHUYE_E2E_SCENARIOS:-}" ]; then
    SCENARIOS="$ZHUYE_E2E_SCENARIOS"
fi
if [ -z "$SCENARIOS" ]; then
    tcase "场景注册表可读" false
    tfinish
    exit 1
fi
echo "  模式:$([ "$MOCK" = "1" ] && echo '自检(mock provider,离线)' || echo '真模型')"
echo "  配置:$TOML"
echo "  场景:$SCENARIOS"

# ── 3. 逐场景执行(一个进程一场景:崩溃 / 超时互相隔离)────────────────────
WORK="$(mktemp -d "${TMPDIR:-/tmp}/zhuye-cli-e2e-suite.XXXXXX")"
cleanup() {
    if [ "$KEEP" = "1" ]; then
        echo "  [keep] 临时目录保留:$WORK"
    else
        rm -rf "$WORK"
    fi
}
trap cleanup EXIT

for scenario in $SCENARIOS; do
    echo
    echo "  ── 场景:$scenario ──"
    SC_DIR="$WORK/$scenario"
    RESULT="$SC_DIR/result.json"
    mkdir -p "$SC_DIR"

    # macOS 无 timeout;lib.sh 的 run_with_timeout 用后台 pid + kill 模拟,
    # 超时返回 124(与 GNU timeout 语义对齐)。
    run_with_timeout "$TIMEOUT" python3 "$HERE/cli_e2e_run.py" \
        --scenario "$scenario" --bin "$BIN" --work "$SC_DIR/ws-root" \
        --toml "$TOML" --result "$RESULT" \
        $([ "$MOCK" = "1" ] && echo --mock) \
        > "$SC_DIR/run.out" 2> "$SC_DIR/run.err"
    rc=$?
    if [ "$rc" = "124" ]; then
        # 超时既可能是端点排队(xhigh 思考 + 全套连跑),也可能是 TUI
        # 挂死 —— 都该被看见,所以判 FAIL;日志尾巴给出定位线索。
        tcase "$scenario · 场景执行未超时(${TIMEOUT}s)" bash -c \
            'printf "超时 %ss:可能是端点排队/慢;也翻 run.err / 屏幕 JSON 确认不是 TUI 挂死\n" "$1"; exit 1' _ "$TIMEOUT"
        tail -5 "$SC_DIR/run.err" 2>/dev/null | sed 's/^/         │ /'
        continue
    fi
    if [ "$rc" != "0" ] && [ "$rc" != "2" ] && [ ! -s "$RESULT" ]; then
        tcase "$scenario · 场景执行器正常产出结果" false
        tail -20 "$SC_DIR/run.err" | sed 's/^/         │ /'
        continue
    fi

    # 逐个检查项折成一条 tcase(名称带场景前缀,便于定位)。
    while IFS=$'\t' read -r status name detail; do
        [ -z "$name" ] && continue
        case "$status" in
            ok)
                tcase "$scenario · $name" true
                ;;
            FAIL)
                tcase "$scenario · $name" bash -c 'printf "%s\n" "$1"; exit 1' _ "$detail"
                ;;
            *)
                echo "  [skip] $scenario · $name —— $detail"
                ;;
        esac
    done < <(python3 - "$RESULT" << 'PYEOF'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as f:
    result = json.load(f)
for item in result.get("checks", []):
    status = item.get("status")
    tag = "ok" if status == "pass" else ("FAIL" if status == "fail" else "SKIP")
    detail = (item.get("detail") or "").replace("\n", " ⏎ ")
    print(f"{tag}\t{item.get('name')}\t{detail}")
PYEOF
    )
done

echo
echo "  结果 JSON 目录:$WORK"
tfinish
