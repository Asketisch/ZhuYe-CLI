# Contributing Guide

<p align="center">  <a href="CONTRIBUTING.md">中文</a> · <b>English</b>  </p>

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
| `770087c` | 指针 883a581 → 770087c:上游 `fix(ci)` 修 `zhuye-core/tests/single_turn.rs:1527` 的 redundant clone(本仓库 `cargo clippy --workspace --all-targets` 因 cargo 自动成员吸收把 submodule 测试目标纳入 lint 面而挂红)+ 覆盖计数刷新;区间内另含 56f8836(审批等待可被回合取消 / 审批落盘 / headless 自动应答)、9aa360f + ce57058(工具×模式×阻塞组合穷举修复与回归测试)、19321c5(Cargo.lock 补录)、4b5f384(docs i18n);协议层(`crates/protocol`)零变更,本仓库无代码适配 |
| `2fb502f` | 指针 770087c → 2fb502f:区间含四项引擎侧新增 —— ① `[[provider_group]]` 模型组(zhuye-config schema + builder,池名 = 组 id,热重载 / failover / `model = "free"` 清单发现自动兼容);② zhuye-llm models.dev 目录(`enrich_model_info` 内嵌快照,未知模型窗口 / 输出上限 / 思考档位自动填充,零接线收益);③ zhuye-skills skills.sh 目录发现(`fetch_index` / `search_entries`,本仓库暂未接线,留待 `/skills` picker 改造);④ zhuye-browser 新 crate(CDP 受控浏览器 + 8 个 browser_* 工具,经 `plugins/browser` 插件安装路径启用,不在本仓库依赖图,根 Cargo.toml 无需镜像)。本仓库代码适配四处:根 `[workspace.dependencies]` 补 `tar`/`flate2` 镜像声明(zhuye-skills 的 skills.sh tarball 解包以 `workspace = true` 继承,跨 workspace 解析落到本根清单,缺声明即 manifest 加载失败)+ `ReflectConfig::active_provider()` 返回类型由 `Option<&'static str>` 改 `Option<String>`(模型组 id 为动态值)→ `/model` 清单路径 `fetch_catalog` / `format_catalog` 调用点改传引用(tui/mod.rs)+ `config_reload::diff_sections` 补 `provider_group` 段比(此前改组配置后热重载 Notice 报「变更段: (无)」;引擎 exec 的 diff_sections 同样漏本段,submodule 只读,TUI 侧先行补上)+ `build_quota_tracker` 遍历 `[[provider_group]]` 显式枚举凭据的 quota 声明(provider 名 = 组 id;组级 api_key wrap 出的 default 凭据 quota 恒 None,无需处理);新增 config_reload ×1 + bootstrap_wiring ×3 Rust 单测;协议层(`crates/protocol`)零变更 |
| `8da90e5` | 指针 2fb502f → 8da90e5(v0.1.15-68):**配置模型 v2 大版本** —— `[[provider_group]]` 更名 `[[model_group]]` 并成为唯一接入单元(固定段 `[anthropic]` / `[openai]` / `[ollama]` 移除,v1 旧配置由 `load` 的迁移逻辑就地转成等价模型组,`[active].provider` → `[active].group`,`[subagent_providers]` → `[subagent_models]`,新增 `config_version` 字段);引擎 exec 的 diff_sections 自本版起已覆盖 `model_group`(2fb502f 行「TUI 先行补」的口径归位)。本仓库代码适配六处:① `build_quota_tracker` 改遍历 `cfg.model_group`(内置三段字段已不存在,组级 base_url 回退逻辑保留,`ProviderGroupSection` → `ModelGroupSection`);② `config_reload::diff_sections` 去 anthropic/openai/ollama 段比、`provider_group` → `model_group`、`subagent_providers` → `subagent_models`,并补 v2 新增段(ask_user_question / prompt_guard / subagents / postgres_session / sse_redis / bridge / voice / dap / acp),保持 TUI 补全版口径;③ bootstrap.rs + tui/mod.rs 共三处 `active_provider()` 改名 `active_group()`(签名不变 `Option<String>`),启动报错文案改「模型组」语汇;④ 根 `[workspace.dependencies]` 的 zhuye-kms 镜像路径跟移(abilities → integrations,上游 `6704a11` 归位);⑤ **门控型 bundled 插件接线**(上游 `7b4230c` 基建):工具注册从手工清单切到单一事实源 `zhuye_tools::register_builtin_tools_with_gates`(TUI 工具面由 17 → 27,补 BackgroundStatus / Worktree ×2 / RequestHumanInput / ToolSearch / NotebookEdit / Brief / GetContextRemaining / Checkpoint / Rewind;web_search / notebook_edit 按 `zhuye_plugin::bundled_tool_gates` 门控注册,`[plugins].disabled_plugins` 显式禁用 = 不进注册表),AskUserQuestion 上限改从 `[ask_user_question]` 段注入,verify-hooks 插件启用 = hooks 注入 `verification` / `test_runner`(build_engine 前),bootstrap_lsp 增 `lsp_plugin_disabled` 参数(`plugins.is_disabled("lsp")`,config 激活语义不变);⑥ 死文件 `cli/models.rs`(未在 cli/mod.rs 声明)同步 `active_group()` 防腐。功能用例:B10 / W10 断言口径改 v2 文案(错误含 `模型组` / `[[model_group]]`),新增 G5(门控默认态零变化)/ G6(`disabled_plugins` 注册门控行为级断言)×2;新增 config_reload ×1(subagent_models 段比)Rust 单测;协议层零变更 |
| `c34c5a1f` | 指针 65c9e8e → c34c5a1f(v0.1.15-72 → -81;引擎 9 提交先落在 Desktop 侧本地克隆,远端 main/develop 暂仍在 65c9e8e,经本地路径互取):**v1.12 挂起交互 + 子代理会话血缘**。引擎侧:① `Event.seq` 会话扇出单调序号(additive,断线重连客户端做缺口检测用);② 协议新变体 `ContextComposition`(上下文组成六类估算,指纹去重)与 `InteractionResolved`(挂起交互任意路径了结通知);③ 挂起交互注册表(`ApprovalContext.pending`,`AgentThread::pending_interactions()` 查询面,plan 审批同表);④ 子代理落独立 JSONL(`SubAgentFactory::set_rollout_base` / `set_workspace`;`SessionMeta` 增 `parent_session_id`、`Fork` 增 `child_session_id`);⑤ zhuye-tools 新依赖 chrono(已在本根清单,无镜像补声明)。本仓库接线五处:`conversion.rs` 两新事件臂(seq 注释声明内嵌直连无缺口面、有意忽略)+ `adapter.rs` 新投影 `InteractionResolved`(五类挂起态按 id 精确清理 —— 审批 / plan 审批 / plan 进入确认 / 问答 / 询问;非 `Answered` 结局补 Notice 说明弹条去向)与 `ContextComposition`(纯数据面入 `context_usage.composition`,不刷屏)+ `/status` 两处实现(枚举通道 `SlashCommand::Status` 与 `SlashOutcome::ShowStatus`)经 `statusline::format_composition` 渲染组成分解 + `bootstrap_wiring::build_m5` 装配点补 `set_rollout_base(default_base())` / `set_workspace`(对齐 exec `bootstrap_m5`,开启子代理会话落盘与 workspace 继承)+ `session ls` 子会话 `↳` 血缘标注 / `session show` 补 `Parent:` 行。测试:conversion ×2 / adapter ×4 / statusline ×1 Rust 单测;功能用例 C11 / C12(血缘标注与 Parent 行)/ T10b(`/status` 组成分解)/ W14 扩展断言(子会话 JSONL 带 parent_session_id 与继承 workspace、父 Fork 回填 child_session_id) |
| `478deb6` | 指针 c34c5a1f → 478deb6(v0.1.16;引擎 4 提交):① `83875a8` **父上下文交接回归修复** —— 子线程落独立 rollout 后 submission_loop 历史装配走 replay 分支,首回合 replay 只有 SessionMeta,`preload_messages`(`context_tail` / pass_context_messages 父上下文交接)被整体丢弃 —— 修复为 rollout 无对话记录时用 preload 作种子历史;测试适配:子/孙会话落独立 JSONL 后 sessions 多文件,parallel / subagent_failure / nested 三文件按场景标记与 Fork 血缘定位父会话文件、resume 目标同步修正;② `e3e92e5` rust stable 1.100 起 `map_unwrap_or` 进默认告警组,四处机械改写(zhuye-tools approval/gate + plan_approval 的 now_ms / created_at_ms、zhuye-core model_call 上下文组成计量)—— 本仓 `-D warnings` clippy 面同样中招,子模块带入即愈;另上游清理 12 个零引用 workspace 依赖声明(上游根清单,本仓镜像面不受影响);③ `467607c` plan 草稿测试夹具写前兜底重建目录(macOS runner 实测 create_dir_all 成功后数微秒内 fs::write ENOENT,疑与并行 Drop 的 remove_dir_all 竞态相关);④ `478deb67` v0.1.16 版本 bump。本仓库代码适配:无(引擎侧自愈);子模块指针即本仓 release 内容)
| `e246299` | 指针 478deb6 → e246299(**main 历史改写后的强制前移**:478deb6 不在新 main 历史中,v0.1.16 的 4 提交以新 SHA 重放(86c35a9 等价),新 main = 86c35a9 + 24 提交,合入 develop 的 v1.x 能力面与 gated-bundled-plugins 分支)。引擎侧关键变化:① 协议新增 6 个 `EventMsg` 变体 —— 流锚点恢复 `StreamRecovery{Started,AnchorSelected,TailDiscarded,RetryStarted}`(additive 诊断族,不改 stream_error / agent_message_reset 语义)、`CacheWarming`(保温诊断推送,无对应 Op)、`McpElicitation`(server 请求宿主用户交互,回执 `Op::McpElicitationResponse`);② `TokenCountEvent` 增 `turn_total` 标记回合累计(与逐次快照一并累计会翻倍,协议文档明示);③ `PromptListEvent` 增 `groups` 两级目录分组(空 = 按 available 平铺退化);④ `zhuye_mcp::McpLifecycleEvent` 增 `Elicitation { event, respond }` 与 `ToolsChanged { server, tools }` 两变体(exec `bootstrap_m6` 有参考接线:waiter 表登记先于发事件 + 300s TTL fail-closed);⑤ S2 `/tasks` 反射路径不再经 `pollster::block_on`。本仓库接线六处:① `conversion.rs` 补 6 新事件臂(StreamRecovery×4 / CacheWarming → Notice 进度行;McpElicitation → Notice 说明已取消)+ `TokenCount` 臂增 `turn_total` 丢弃(HUD 最近一次调用与 /status 按模型·费用累计防翻倍)+ `PromptList` 臂附分组摘要;② `bootstrap_wiring.rs` MCP 转发 task 补双臂 —— `ToolsChanged` 按注册表现状差集反注册消失工具、新工具绑活连接重挂(提取纯函数 `removed_mcp_tools`),`Elicitation` 因本构建无交互面按 fail-closed 立即 cancel 回执(不登记 waiter 不等 TTL,事件仍转发保证可见性;已知限制见 COVERAGE.md D16);③ 移除零引用依赖 `pollster`(本仓 crate + 根声明);④ `[workspace.dependencies]` 无需补声明 —— 新增引擎成员(zhuye-eval / zhuye-computer-use / zhuye-browser / zhuye-shell / zhuye-git-tools / runtime/zhuye / runtime/zhuye-py)均不在本仓消费闭包,既有镜像清单解析通过;⑤ 测试:conversion ×6(McpElicitation form/link / CacheWarming executed·skipped / StreamRecovery 四事件族)+ bootstrap_wiring ×2(差集)+ mod.rs ×1(turn_total 丢弃)+ 既有 5 处构造字面量补新字段(PromptListEvent.groups ×2、TokenCountEvent.turn_total ×3);⑥ 文档同步:crate 篇契约 11、tui 篇契约 7、COVERAGE.md §1 矩阵行 + D16 |
| `e52bfda` | 指针 e246299 → e52bfda(v0.1.17;引擎 138 提交,含 main 强制前移后的能力面合流)。引擎侧关键变化:① `refactor(naming)` **全量更名 Reflect → Zhuye** —— 配置类型 `ReflectConfig` → `ZhuyeConfig`、`HooksConfig::from_reflect_section` → `from_zhuye_section`(纯更名,签名未变);② 协议 `Op::Compact` 增 `instructions: Option<String>` 字段(v1.x `/compact <focus>` 附加指示,`None` = 普通压缩,wire 兼容);③ 协议新增 2 个 `EventMsg` 变体 —— `ToolExecutionUncertain`(恢复时发现结果未知的工具执行)与 `TaskBudgetSnapshot`(任务树预算快照);④ browser 后端改用 `agent-browser` **嵌套 submodule**(`third_party/agent-browser`,替换手写 CDP;JS 工具面,不入 cargo 依赖图);⑤ 修复类:rapid-refill 熔断失效与手动强制压缩静默失效、Seatbelt 放行 Mach IPC、subagent 支持续派已完成子代理、KMS/LSP/MCP/shell Windows 整批修复;⑥ 引擎侧重组(能力覆盖层测试分六子目录、zhuye-core misc.rs 拆分、model_discovery 归位、能力覆盖矩阵 401 → 432),本仓消费面不受影响。本仓库接线四处:① 全仓标识符随更名(`ReflectConfig` → `ZhuyeConfig` ×5 文件、`from_zhuye_section` ×1;bootstrap_wiring 两处 `&!`/`len on !` 系该类型解析失败的级联推断错误,随更名自愈,无独立适配);② `Op::Compact` 两处 emit(tui/mod.rs 枚举通道与 legacy `SlashOutcome::Compact` 路径)补 `instructions: None`(本构建 `/compact` 无参数解析,普通压缩语义不变,`<focus>` 附加指示留待后续);③ `conversion.rs` 补两新事件臂 —— `ToolExecutionUncertain` → Notice(与 StreamRecovery 同族,流恢复留痕)/ `TaskBudgetSnapshot` → 有意丢弃(回合级预算遥测,TUI 无消费面,token 用量走 TokenCount);④ `config_reload::diff_sections` 补 `cache_warming` 段比(引擎 exec 名单已有、本仓漏,改 `[cache_warming]` 后热重载 Notice 报「变更段: (无)」)。测试:conversion ×2 + config_reload ×1 Rust 单测;能力覆盖层(run_all functional)与覆盖自审全绿。嵌套 submodule 落地后消费需 `git submodule update --init --recursive`。文档同步:tui 篇契约 7、COVERAGE.md §1 诊断事件族行 |

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
