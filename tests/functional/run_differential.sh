#!/usr/bin/env bash
# 明确启用时执行 TUI 版本差分(tui.contract.differential)。
# 需要提供两个被测版本的 zhuye-cli 二进制路径;结果写入 target/verification-tui。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
: "${ZHUYE_DIFF_BASELINE:?需要指定基线二进制}"
: "${ZHUYE_DIFF_CANDIDATE:?需要指定候选二进制}"
export ZHUYE_DIFF_BASELINE ZHUYE_DIFF_CANDIDATE
: "${ZHUYE_DIFF_OUTPUT:=$ROOT/target/verification-tui}"
export ZHUYE_DIFF_OUTPUT
# 复用能力覆盖层夹具(pyte 驱动真实 PTY),只跑真实差分用例。
exec python3 -m pytest "$ROOT/tests/functional/test_differential.py" -k 真实差分 -q --tb=short
