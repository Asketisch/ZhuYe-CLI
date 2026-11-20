#!/usr/bin/env bash
# tests/rust/build_tooling.sh —— 构建工具面(Makefile 清理目标)的语义断言。
#
# 覆盖:
#   make gc         —— 清理面覆盖产物根下全部 profile 的 incremental 目录,
#                      且**不**误伤 .fingerprint / deps / examples / release 二进制
#   make gc-deep    —— 深清理:在 gc 之外再清 deps / .fingerprint / examples
#   产物根覆盖      —— TARGET_DIR 可覆盖(默认跟随 CARGO_TARGET_DIR)
#
# 断言方式:在临时产物根上跑真实 make,再检查文件系统结果 ——
# 断言到「哪些目录被清、哪些被保留」这一行为级,而不是「退出码 0」。
#
# 不依赖 release 二进制;全程不触碰仓库真实的 target/。
# 退出码 0 = 全部通过。

set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/../lib.sh"

cd "$ROOT"

# make 不存在时跳过(Windows 侧无 GNU make;CI 的 functional 套件本就不在 Windows 跑)。
if ! require_cmd make; then
    echo "  [SKIP] 未找到 make —— 构建工具面用例跳过(该平台无 GNU make)"
    exit 0
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/zhuye-cli-build-tooling.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

FAKE_TARGET="$TMP/target"

# ok <名称> <0/1 结果>:记一条通过/失败(文件系统断言用)。
ok() {
    if [ "$2" -eq 0 ]; then
        PASS_COUNT+=1; echo "  [ok]   $1"
    else
        FAIL_COUNT+=1; FAIL_NAMES+=("$1"); echo "  [FAIL] $1"
    fi
}

# seed_fake_target:造一棵假的产物目录树,覆盖所有会影响清理结果的布局:
#   <profile>/incremental/            —— release-fast 等 profile(dev profile 不产生)
#   <triple>/<profile>/incremental/   —— 交叉编译(scripts/build.sh --target=<triple>)
#   <profile>/.fingerprint/            —— cargo 的新鲜度凭据
#   <profile>/deps/ + examples/        —— 已编译依赖与示例产物
#   <profile>/<bin>                    —— release 二进制(测试套件依赖它)
seed_fake_target() {
    rm -rf "$FAKE_TARGET"
    mkdir -p "$FAKE_TARGET/debug/incremental/session-a"
    mkdir -p "$FAKE_TARGET/release-fast/incremental/session-b"
    mkdir -p "$FAKE_TARGET/x86_64-unknown-linux-gnu/debug/incremental/session-c"
    mkdir -p "$FAKE_TARGET/debug/.fingerprint/lib-foo"
    mkdir -p "$FAKE_TARGET/debug/deps" "$FAKE_TARGET/debug/examples"
    mkdir -p "$FAKE_TARGET/release"
    : > "$FAKE_TARGET/debug/deps/libfoo.rlib"
    : > "$FAKE_TARGET/debug/examples/foo-example"
    : > "$FAKE_TARGET/release/zhuye-cli"
}

# ── 1. make gc:清增量目录,不误伤已编译产物 ─────────────────────────────

seed_fake_target

make gc TARGET_DIR="$FAKE_TARGET" > "$TMP/gc.log" 2>&1
GC_RC=$?
assert_exit "make gc 退出码为 0" 0 "$GC_RC"

ok "make gc 清掉 dev profile 的 incremental" \
    "$([ ! -e "$FAKE_TARGET/debug/incremental" ] && echo 0 || echo 1)"
ok "make gc 清掉 release-fast profile 的 incremental" \
    "$([ ! -e "$FAKE_TARGET/release-fast/incremental" ] && echo 0 || echo 1)"
ok "make gc 清掉交叉编译 target 的 incremental" \
    "$([ ! -e "$FAKE_TARGET/x86_64-unknown-linux-gnu/debug/incremental" ] && echo 0 || echo 1)"

# 回归护栏:删 .fingerprint 会让 cargo 把所有依赖判为「从未构建」
# (DirtyReason::FreshBuild)而下次 build 全量重编,代价远大于它占的体积 ——
# gc 不得碰它。
ok "make gc 保留 .fingerprint(删它会导致下次全量重编)" \
    "$([ -e "$FAKE_TARGET/debug/.fingerprint/lib-foo" ] && echo 0 || echo 1)"
ok "make gc 保留 deps 里已编译的 rlib" \
    "$([ -e "$FAKE_TARGET/debug/deps/libfoo.rlib" ] && echo 0 || echo 1)"
ok "make gc 保留 examples 产物" \
    "$([ -e "$FAKE_TARGET/debug/examples/foo-example" ] && echo 0 || echo 1)"
ok "make gc 保留 release 二进制(测试套件依赖它)" \
    "$([ -e "$FAKE_TARGET/release/zhuye-cli" ] && echo 0 || echo 1)"

# ── 2. 产物根默认跟随 CARGO_TARGET_DIR ─────────────────────────────────

# 把 Makefile 复制到临时目录再跑:这样即使配方硬编码了相对路径 target/,
# 落点也只是临时目录,不会碰到仓库真实的 target/。
# 断言「CARGO_TARGET_DIR 指向哪就清哪」—— 清理面必须跟着 cargo 的产物位置走,
# 不能钉死在仓库根的 target/。
mkdir -p "$TMP/mk"
cp "$ROOT/Makefile" "$TMP/mk/Makefile"

seed_fake_target

CARGO_TARGET_DIR="$FAKE_TARGET" make -C "$TMP/mk" gc > "$TMP/gc-cargo-target-dir.log" 2>&1
assert_exit "make gc 在 CARGO_TARGET_DIR 下退出码为 0" 0 "$?"

ok "make gc 跟随 CARGO_TARGET_DIR 清掉 incremental" \
    "$([ ! -e "$FAKE_TARGET/release-fast/incremental" ] && echo 0 || echo 1)"
ok "make gc 跟随 CARGO_TARGET_DIR 时同样保留 .fingerprint" \
    "$([ -e "$FAKE_TARGET/debug/.fingerprint/lib-foo" ] && echo 0 || echo 1)"

# ── 3. 产物根不存在时不报错(未匹配的 glob 被 rm -f 静默忽略)────────────

make gc TARGET_DIR="$TMP/does-not-exist" > "$TMP/gc-empty.log" 2>&1
assert_exit "make gc 在产物根不存在时退出码仍为 0" 0 "$?"

# ── 4. make gc-deep:深清理 deps / .fingerprint / examples ───────────────

make gc-deep TARGET_DIR="$FAKE_TARGET" > "$TMP/gc-deep.log" 2>&1
assert_exit "make gc-deep 退出码为 0" 0 "$?"

ok "make gc-deep 清掉 deps" \
    "$([ ! -e "$FAKE_TARGET/debug/deps" ] && echo 0 || echo 1)"
ok "make gc-deep 清掉 .fingerprint" \
    "$([ ! -e "$FAKE_TARGET/debug/.fingerprint" ] && echo 0 || echo 1)"
ok "make gc-deep 清掉 examples" \
    "$([ ! -e "$FAKE_TARGET/debug/examples" ] && echo 0 || echo 1)"

# ── 5. help 与目标同步 ─────────────────────────────────────────────────

make help > "$TMP/help.txt" 2>&1
assert_contains "make help 列出 gc 目标" "gc " "$TMP/help.txt"
assert_contains "make help 列出 gc-deep 目标" "gc-deep" "$TMP/help.txt"

tfinish "build_tooling"
