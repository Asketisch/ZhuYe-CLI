# Contributing Guide

<p align="center">  [中文](CONTRIBUTING.md) · **English**  </p>

Thanks for your interest in ZhuYe-CLI! This repository contains **only the
TUI presentation layer** (`crates/zhuye-cli`); the core engine is reused from
[Reflect-Agent](https://github.com/Asketisch/Reflect-Agent) via a git
submodule. Please read the architecture overview in the
[README](README.md) first.

## Development Environment

```bash
git clone --recursive https://github.com/Asketisch/Reflect-TUI.git
cd ZhuYe-CLI
cargo build --release          # produces target/release/zhuye-cli
cargo test -p zhuye-cli      # TUI-side tests
```

Configuration lives in `~/.zhuye/config.toml` (provider / MCP / LSP / hooks /
permissions), or via the `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` environment
variables. For an offline experience use `ZHUYE_MODEL=mock/mock-1` (built-in
mock provider, zero network).

## Pre-Submit Checks

CI (ubuntu + macOS) runs the following; please pass them locally first:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./tests/run_all.sh functional     # capability coverage layer (real binary + pty behavior assertions)
python3 tests/tools/coverage_audit.py --strict   # coverage audit
```

> **New features or behavior changes must be submitted together with their
> test scripts.** If you change externally visible TUI behavior (rendered
> text / key routing / overlays / slash commands / conversation turns /
> session persistence / CLI output), there must be a corresponding case in
> `tests/functional/`, asserting at the **behavior level** (visible screen
> text / persisted file contents / exit codes). "Exit code 0" or "does not
> crash" pseudo-assertions are not accepted. A new slash command also needs
> a new row in the `MATRIX` of `tests/functional/test_slash.py`, otherwise
> the coverage audit fails outright. The full checklist (change surface →
> test location) is in the "development process checklist" section of
> [AGENTS.md](AGENTS.md) (in Chinese).

The last two items gate the **functional surface** and complement the first
two **compile-surface** checks:

- `./tests/run_all.sh functional` launches a real `zhuye-cli` process and
  uses pty + screen snapshots to assert "what the user actually saw and what
  actually landed on disk", covering the runtime outcomes of all 84 slash
  commands. See [tests/README.md](tests/README.md) (in Chinese).
- `coverage_audit.py` parses the declared slash commands out of the Rust
  source and diffs them against the test matrix — **add a command without a
  test case and this fails immediately**. When changing slash commands, add
  a row to the `MATRIX` in `tests/functional/test_slash.py`.

> Prerequisites: `cargo build --release -p zhuye-cli` (the entry script
> builds it automatically if missing), Python 3 + `pytest` + `pyte`
> (`pip install pytest pyte`). Missing must-have dependencies such as
> pytest / pyte count as infrastructure errors, not as passing.

Note: clippy lint sets differ across rustc versions. If your toolchain is
newer than CI's, the bar is "no errors from `cargo check` + no new warnings".

## Code Conventions

- **Layering**: this crate only handles the presentation layer — event loop,
  layout, rendering, protocol-event adaptation (`tui/conversion.rs` is the
  single conversion boundary from protocol events to UI events). Engine
  capabilities are invoked through the submodule's library crates; engine
  logic is never re-implemented inside the TUI.
- **Wiring alignment**: capability assembly in bootstrap uses
  `zhuye-exec/src/bootstrap.rs` (headless) as its reference — before adding
  an engine capability to the TUI, check how it is wired there.
- **Comments**: explain "why", not "what". This project intentionally keeps
  a high comment density (recording pitfalls and design trade-offs), but
  only informative ones. **All code comments must be written in Chinese.**
- **Protocol evolution**: changes to the wire protocol (Submission / Op /
  EventMsg) belong to the Reflect-Agent repository; this repository only
  does consumer-side adaptation.
- **Slash commands**: this project has **two dispatch channels** (a legacy
  text channel and an enum channel) with inconsistent behavior — check both
  when changing commands, and register the runtime outcome in the `MATRIX`
  of `tests/functional/test_slash.py` and in [tests/COVERAGE.md](tests/COVERAGE.md)
  §S (both in Chinese).
- **Bilingual docs**: user-facing documents (README / docs/ / SECURITY /
  CODE_OF_CONDUCT / this file) are paired Chinese-English split files
  (`x.md` + `x.en.md`, with language switch bars linking to each other);
  after changing a Chinese version you **must sync the corresponding
  `.en.md`** — the English versions are outside CI gating but must be
  maintained together; long-term drift is not allowed. The AGENTS.md system
  and internal tests docs stay Chinese-only. (This rule's authoritative
  statement is in the Chinese version.)

## Submodule Workflow

The core engine (Reflect-Agent) is brought in via the git submodule
`zhuye-agent/`; this repository only does consumer-side adaptation. The
standard upgrade flow:

```bash
git submodule update --remote zhuye-agent   # pull the latest pointer
git -C zhuye-agent log --oneline <old-pointer>..HEAD   # read every change
cargo check -p zhuye-cli --all-targets       # verify the compile surface
cargo test -p zhuye-cli && cargo clippy -p zhuye-cli --all-targets -- -D warnings
git add zhuye-agent && git commit            # commit pointer and adaptation together
```

Key points:

- **Pointer changes must be committed together with the code adaptation** —
  moving the pointer without adapting the code leaves a repository that does
  not compile.
- After an upgrade, go through every new / changed public API: this
  repository directly depends on `zhuye-core`, `zhuye-protocol`,
  `zhuye-rollout`, `zhuye-telemetry`, `zhuye-config`, `zhuye-compact`, and
  others; API changes in these crates need corresponding consumption points
  here.
- **Prefer shared APIs**: when the submodule provides reading / aggregation
  capabilities (e.g. `zhuye_rollout::session_stats`), do not re-implement a
  copy of the aggregation logic here, or the two sides will drift.
- This repository's `AGENTS.md` records this project's architecture and
  constraints; the submodule-side notes live in `zhuye-agent/AGENTS.md`
  (both in Chinese).

### Wiring Log

> The table below is a pointer log kept in Chinese for precision — it is
> intentionally **not translated**. See [CONTRIBUTING.md](CONTRIBUTING.md)
> for the original.

| submodule pointer | wiring summary (kept in Chinese) |
| --- | --- |
| `5f45f0a` | v0.1.12 指针适配:根 Cargo.toml 镜像 `[workspace.lints]`(核心 crate 自 v0.1.9 起以 `[lints] workspace = true` 继承,跨 workspace 解析落到外层根清单,缺定义即 manifest 加载失败);补声明 zhuye-kms / zhuye-audit / zhuye-secrets / zhuye-fs / zhuye-git / zhuye-wasm / facade `zhuye`,移除已更名消亡的 `reflect` 死路径;外部依赖版本对齐(pyo3 0.23→0.30、reqwest 补 gzip、新增 rusqlite / sha2 / aes-gcm / hkdf / wasmtime(-wasi) 30.0.1 / proptest);代码适配三处 —— `M4Deps` 新字段 `skill_dirs` / `bundled_skills`(TUI 与 exec 同口径:内置 skill 包默认关闭,`[active] bundled_skills = true` 显式开启),`PermissionRule.resource_glob`(TUI always-allow 不带资源上下文,填 `None`),`convert_event` 补 `SideChannelStarted/Finished/Status` + `ExtensionsRescanned` 分支(Notice 行透出,交互式频道视图留待后续) |
| `34238dd` | CI 修复批次:reflect-sandbox 的 seatbelt 相关代码(`seatbelt_argv` / `seatbelt_profile` / 原子序号常量)补 `#[cfg(target_os = "macos")]` 门控,`seatbelt_argv` 另补非 macOS 占位实现(调用方按 status 运行时分派,占位实现运行时不可达),修复 Windows 编译失败(`std::os::unix` 不存在);macOS/Linux 行为不变,本仓库无消费点适配。该修复已以 `e7bbb4b` 并入 zhuye-agent main(烛夜更名后路径为 `crates/resources/zhuye-sandbox/`);本仓库指针暂留 `34238dd` —— 更名后结构本仓库尚不能消费,指针随外层更名适配一并前移,临时分支已删除 |
| `e4560bc` | 修复批次(D1–D7 收口):会话恢复回放换 submodule 共享 API —— `reader::replay` 升级后支持任意日期桶 + 轮转副本合并 + 文件名/内部 id 双入口,删除本仓库此前维护的全盘扫描 workaround(`resume.rs::collect_session_files`);`JsonlRolloutWriter` 恢复后续写原日期目录的原文件(会话不再被拆成两份);「会话不存在」仍显式报错(共享 API 返回空 Vec,包装层兜底) |
| `7912323` | **未接线 submodule 能力全量接线**(v1.5 / v1.7):`EventMsg::ToolCallArgsDelta` 流式工具参数(live 区渐进预览 + 参数摘要进调用行)、`EventMsg::Routing` 路由事件(failover / 冷却 / `SpecFallback` 跨模型降级)、`reflect_rollout::search_sessions` 跨会话全文搜索(新增 `session search`)、`reflect_config::resolve_model_spec` + `[model_discovery]`(`model = "free"` 运行时解析,配 `routing_policy_with` 传真实 spec)、`reflect_llm::model_catalog` provider 模型清单 + 免费判定(`/model` 无参列出)、`reflect_goal::GoalState` 权威读口(`/goal` 无参显示状态 + `--verify` / `--budget` 选项)、`reflect_config::init_exporter` OTLP 分析遥测、`reflect_config::ConfigWatcher` 配置热重载、`reflect_mcp::curated_catalog` MCP 精选目录(`/mcp` overlay)、`reflect_lsp::DiagnosticRegistry` 诊断(`/lsp`)、`AgentConfig::todos_slot` 待办清单(`/todo`)、`reflect_sandbox::OsSandbox::status_line` 沙箱状态(`/status`)、`reflect_stream::workspace_sync` 工作区克隆(新增 `workspace clone`)、`reflect_discussion::cli::run` 多 agent 讨论(新增 `discussion run`)、`reflect_integration::security_scan` 安全扫描(新增 `security audit`)、`reflect_config::TuiNotificationsSection` 终端通知开关与通道(`[notifications.tui]` 真正生效);同时修 `[sandbox].os_level` 改 `Option<bool>` 带来的编译断点 |
| `d771c9e` | 动态 model 解析接线(`model = "free"` → 启动期经 `reflect_config::resolve_model_spec` 从 provider 清单挑出真实 id,不再拿占位串 `provider/free` 去打请求)+ routing policy 改用 `routing_policy_with(已解析 spec)`(同步版 `routing_policy()` 会让占位串盖掉真实 id)。`SpecSlot.fallbacks` 跨模型降级链由引擎侧接通,TUI 沿用 `EventMsg::Routing` 兜底分支不新增 UI |
| `e4bc182` | 模型级 token 统计(`TokenCountEvent.model` → HUD / `/status` 按模型用量)、`reflect_rollout::session_stats` / `all_model_usage` 接入 `session show` / 新增 `session stats`、`[telemetry].redact` / `max_bytes` / `record_tool_schemas` 真正生效、span 日志读取侧(新增 `traces spans`)、会话文件定位改走共享 API(文件名 thread id 与内部 `session_id` 可能不同) |
| `46dc156` | 提示词分层库接入 + `/prompt` 运行时管理 |
| `3cb791f` | `fetch_update` 弃用告警收口 + CI 编译修复 |
| `8564f66` | 安全边界与正确性收口(沙箱逃逸 / 路径穿越 / Plan mode) |
| `af0483b` | 持久 shell 状态 + `exec --image` 多模态 + `web_fetch` DNS rebinding 防护 |
| `bb2eaeb` | Linux CI 全绿(landlock 降级重试 + 环境自适应测试) |
| `d9b88de` | `/permissions` 弹窗修复 + MCP Prompts 工具接入 |
| `b625b5e` | `/rewind` 弹窗补齐 + clippy 门槛修复 |
| `ce57d48` | 核心审批设置用例改用无副作用 Prompt 探针,消除 Windows 无 OS 沙箱后端时的平台耦合;TUI 公开 API 与行为零变更,仅接线测试修复 |
| `e383206` | 核心文件任务存储的 Windows 文件锁修复:锁文件原以 `append(true)` 打开,Rust std 会刻意剔除 `FILE_WRITE_DATA`,句柄不带 `GENERIC_READ`,而 Windows 的 `LockFileEx` 要求句柄具备读权限 —— 加锁直接返回 `ACCESS_DENIED`(code 5),表现为文件任务后端在 Windows 上完全不可用。改为 `read + write` 打开后跨进程串行化恢复正常;TUI 公开 API 与行为零变更,仅接线上游 bug 修复 |
| `c728da8` | 上游补充:锁文件打开方式显式声明 `truncate(false)`(消除 `clippy::suspicious_open_options`,行为不变);本仓库无适配 |
| `9332aa9` | develop 分支接线(v0.1.8-39,**重叠功能全面改用 submodule**):①`/checkpoint` 的 git 实现收编 —— 删除本地 `git_checkpoint.rs`,改调 `zhuye_git::{create,list,rewind}`(前缀更名 `reflect-checkpoint:`→`zhuye-checkpoint:`、`reflect-cp/`→`zhuye-cp/`,见 COVERAGE.md D15);②分支 / PR 探测收编 —— 删除本地 `branch_summary.rs`(499 行),`zhuye_git::branch` 跑在 `CommandRunnerBridge` 上(本地 `WorkspaceCommandExecutor` app-server 传输边界保留,孤儿规则经借用型 newtype 桥接);③assistant git 指令解析收编 —— 删除本地 `git_action_directives.rs`,改用 `zhuye_git::parse_assistant_markdown`;④home 解析收编 —— 删除本地 `utils_home_dir.rs`,`find_reflect_home()` 五处消费点改 `zhuye_config::data_dir()` 单一事实源(多 getpwuid 兜底);⑤原子写收编 —— 导出落盘 / 提示词历史 / 主题持久化 / sixel 缓存四处手写 temp+rename 改 `zhuye_fs::write_atomic`(失败保全 + symlink 穿透);⑥协议三事件接线 —— `SideChannelStarted/Finished` 以 Notice 进对话流,`SideChannelStatus` 查询应答丢弃;⑦根 `[workspace.dependencies]` 补 `zhuye-fs`/`zhuye-git` 镜像声明,删除已不存在的 `reflect` crate 声明;新增 side-channel ×3 + 桥接 ×2 Rust 单测 |
| `411e282` | 核心 zhuye-git 命令超时 Windows 整树终止(Job Object,KILL_ON_JOB_CLOSE):孙进程不再逃逸,与 Unix `setpgid`+`kill(-pgid)` 对等;`timed_out_process_cannot_write_after_return` 的 Windows 恒红根因,本仓库无适配 |
| `cd4ad54` | 核心 zhuye-lsp / zhuye-mcp stdio 子进程 `env_clear` 后透传 Windows 运行时候选变量(SystemRoot/windir/ComSpec/PATHEXT 等):缺 SystemRoot 时 node.exe 等 CRT 程序在输出任何字节前崩溃,表现为连接 EOF/Cancelled 而非 spawn 错误;本仓库无适配 |
| `8e72bc4` | 核心 zhuye-shell 会话 kill Windows 整树终止(Job Object)+ env_clear 分支同补运行时变量;`zhuye exec`/常驻 shell 池在 Windows 上 kill 语义与 Unix 对齐;本仓库无适配 |
| `0cca4c6` | 核心 zhuye-telemetry / zhuye-audit 文件锁与追加句柄带读权限:Windows `LockFileEx` 要求 GENERIC_READ/GENERIC_WRITE,纯 append 句柄加锁直接 ACCESS_DENIED(code 5),`activity.rs` 5 个落盘单测的 Windows 恒红根因(此前被 zhuye-git 树杀卡死掩盖,从未跑到该二进制);与 `1c09f6a4→c728da8e` 的 zhuye-task 等三处同一口径的补漏;本仓库无适配 |
| `8abd4ed` | 核心 zhuye-tools safe_write 原子改名切到 FileRenameInformationEx(类 3 拒绝句柄根→Windows code 87,write/edit/plan_write 落盘恒红)+ plan_write 绝对路径按分量判定;本仓库无适配 |

## Commits / PRs

- Commit messages use the concise `type: summary` style (feat / fix / docs /
  test / chore / refactor). Note that commit messages in this repository are
  written in Chinese (see the language policy in the README).
- PRs should describe motivation, changes, and how they were verified (which
  tests ran / manual smoke steps).
- PRs involving submodule pointer changes must call that out explicitly.

## Pre-Release Review

Before a release, run one full review pass covering the **compile surface +
behavior surface + coverage audit**:

```bash
make release-check
```

It runs, in order:

1. `./tests/run_all.sh` — the Rust compile surface (`cargo test` /
   `clippy -D warnings` / `fmt` / `doc`) + the capability coverage layer
   (215 behavior assertions against the real binary over a pty + the CLI
   argument matrix);
2. `tests/tools/coverage_audit.py --strict` — confirms that every declared
   slash command has a behavior-level test case.

If any of these fails, you **should not release**. The functional coverage
matrix (including known product limitations, known defects, and an honest
list of uncovered items) is in [tests/COVERAGE.md](tests/COVERAGE.md) (in
Chinese).

> Releases are built with `./scripts/build.sh --dist`, which only builds and
> packages — it does **not** run tests. Review and packaging are two steps:
> first `make release-check`, then `make dist`. See
> [docs/release.md](docs/release.md) for the full pipeline.

## Reporting Issues

- When opening an issue, include: OS, terminal (iTerm/tmux/...), repro
  steps, and the relevant snippet of `~/.zhuye/logs/tui-<date>.log`
  (remember to redact secrets).
- For rendering issues attach a screenshot or a screen-text dump; for
  crashes attach the full panic.

## Code of Conduct

By participating in this project you agree to abide by the
[Code of Conduct](CODE_OF_CONDUCT.md).
