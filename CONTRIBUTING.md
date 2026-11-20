# 贡献指南

<p align="center">  <b>中文</b> · <a href="CONTRIBUTING.en.md">English</a>  </p>

感谢关注 ZhuYe-CLI！本仓库只包含 **TUI 表现层**（`crates/zhuye-cli`）；
核心引擎由 [Reflect-Agent](https://github.com/Asketisch/Reflect-Agent) 通过
git submodule 复用。请先读 [README](README.md) 的架构说明。

## 开发环境

```bash
git clone --recursive https://github.com/Asketisch/Reflect-TUI.git
cd ZhuYe-CLI
cargo build --release          # 产物在 target/release/zhuye-cli
cargo test -p zhuye-cli      # TUI 侧测试
```

配置：`~/.zhuye/config.toml`（provider / MCP / LSP / hooks / permissions），
或 `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` 环境变量。离线体验可用
`ZHUYE_MODEL=mock/mock-1`（内置 mock provider，零网络）。

## 提交前检查

CI（ubuntu + macOS）会跑以下几项，请本地先过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./tests/run_all.sh functional     # 能力覆盖层（真二进制 + pty 行为断言）
python3 tests/tools/coverage_audit.py --strict   # 覆盖自审
```

> **新功能或行为修改必须与测试脚本一起提交。** 改了 TUI 对外可见的行为
> （渲染文本 / 按键路由 / 弹层 / slash 命令 / 对话轮次 / 会话落盘 /
> CLI 输出），必须在 `tests/functional/` 有对应用例，且断言到**行为级**
> （屏幕可见文本 / 落盘文件内容 / 退出码），不接受「退出码 0」「不崩」
> 这类伪断言。新增 slash 命令还要在 `tests/functional/test_slash.py` 的
> `MATRIX` 补一行，否则覆盖自审直接失败。完整检查表（改动面 → 用例落点）
> 见 [AGENTS.md](AGENTS.md) 的「开发流程检查表」。

后两项是**功能面**的门禁，与前两项的**编译面**互补：

- `./tests/run_all.sh functional` 起真实的 `zhuye-cli` 进程，用 pty +
  屏幕快照断言「用户真的看到了什么、真的落了什么盘」，并覆盖全部
  84 条 slash 命令的运行时结局。详见 [tests/README.md](tests/README.md)。
- `coverage_audit.py` 从 Rust 源码解析出声明的 slash 命令，与测试矩阵做
  差集 —— **新增命令忘了加用例，这里直接失败**。改 slash 命令时，
  `tests/functional/test_slash.py` 的 `MATRIX` 要同步补一行。

> 前置：`cargo build --release -p zhuye-cli`（入口脚本会自动补建）、
> Python 3 + `pytest` + `pyte`（`pip install pytest pyte`）。
> 缺 pytest / pyte 等必测依赖时判基础设施错误，不算通过。

注意：不同 rustc 版本的 clippy lint 集有差异，若你的工具链比 CI 新，
以 `cargo check` 无错误 + 不新增警告为底线。

## 代码约定

- **分层**：本 crate 只负责表现层 —— 事件循环、布局、渲染、协议事件适配
  （`tui/conversion.rs` 是协议事件 → UI 事件的唯一转换边界）。引擎能力
  通过子模块库 crate 调用，不在 TUI 里重新实现引擎逻辑。
- **接线对齐**：bootstrap 的能力装配以 `zhuye-exec/src/bootstrap.rs`
  （headless）为参照物；给 TUI 加引擎能力时先看那边怎么接。
- **注释**：解释「为什么」，不复述「是什么」。本项目注释密度偏高是有意
  为之（记录踩坑与设计取舍），但只写有信息量的。
- **协议演进**：wire 协议（Submission / Op / EventMsg）的变更属于
  Reflect-Agent 仓库；本仓库只做消费端适配。
- **slash 命令**：本项目有**两条分发通道**（legacy 文本通道与枚举通道），
  行为不一致 —— 改命令时两条都要看，并把运行时结局登记进
  `tests/functional/test_slash.py` 的 `MATRIX` 与
  [tests/COVERAGE.md](tests/COVERAGE.md) §S 组。
- **文档双语**：用户可见文档（README / docs/ / SECURITY /
  CODE_OF_CONDUCT / 本文件）为中英分文件成对（`x.md` + `x.en.md`，顶部
  语言切换条互指）；改了中文版须**同步更新对应 `.en.md`** —— 英文版不在
  CI 门禁范围，但必须一起维护，不允许长期漂移。AGENTS.md 体系与 tests
  内部文档保持纯中文，不做双语。

## Submodule 工作流

核心引擎（Reflect-Agent）以 git submodule 形式引入 `zhuye-agent/`，
本仓库只做消费端适配。升级 submodule 的标准流程：

```bash
git submodule update --remote zhuye-agent   # 拉取最新指针
git -C zhuye-agent log --oneline <旧指针>..HEAD   # 逐条读变更
cargo check -p zhuye-cli --all-targets       # 确认编译面
cargo test -p zhuye-cli && cargo clippy -p zhuye-cli --all-targets -- -D warnings
git add zhuye-agent && git commit            # 指针与代码适配一起提交
```

要点：

- **指针变更必须与代码适配一起提交** —— 只改指针不改代码会留下编译不过的仓库。
- 升级后逐条核对新增 / 变更的公开 API：本仓库的直接依赖是 `zhuye-core`、
  `zhuye-protocol`、`zhuye-rollout`、`zhuye-telemetry`、`zhuye-config`、
  `zhuye-compact` 等，这些 crate 的 API 变更需要在本仓库有对应消费点。
- **消费点优先走共享 API**：submodule 提供的读取 / 聚合能力（如
  `zhuye_rollout::session_stats`），不要在本仓库复刻一份聚合逻辑，
  否则两边会漂移。
- 本仓库的 `AGENTS.md` 记录本项目的架构与约束,submodule 侧的说明见
  `zhuye-agent/AGENTS.md`。

### 接线记录

| submodule 指针 | 接线内容 |
| --- | --- |
| `5f45f0a` | v0.1.12 指针适配:根 Cargo.toml 镜像 `[workspace.lints]`(核心 crate 自 v0.1.9 起以 `[lints] workspace = true` 继承,跨 workspace 解析落到外层根清单,缺定义即 manifest 加载失败);补声明 zhuye-kms / zhuye-audit / zhuye-secrets / zhuye-fs / zhuye-git / zhuye-wasm / facade `zhuye`,移除已更名消亡的 `reflect` 死路径;外部依赖版本对齐(pyo3 0.23→0.30、reqwest 补 gzip、新增 rusqlite / sha2 / aes-gcm / hkdf / wasmtime(-wasi) 30.0.1 / proptest);代码适配三处 —— `M4Deps` 新字段 `skill_dirs` / `bundled_skills`(TUI 与 exec 同口径:内置 skill 包默认关闭,`[active] bundled_skills = true` 显式开启),`PermissionRule.resource_glob`(TUI always-allow 不带资源上下文,填 `None`),`convert_event` 补 `SideChannelStarted/Finished/Status` + `ExtensionsRescanned` 分支(Notice 行透出,交互式频道视图留待后续) |
| `34238dd` | CI 修复批次:reflect-sandbox 的 seatbelt 相关代码(`seatbelt_argv` / `seatbelt_profile` / 原子序号常量)补 `#[cfg(target_os = "macos")]` 门控,`seatbelt_argv` 另补非 macOS 占位实现(调用方按 status 运行时分派,占位实现运行时不可达),修复 Windows 编译失败(`std::os::unix` 不存在);macOS/Linux 行为不变,本仓库无消费点适配。该修复已以 `e7bbb4b` 并入 zhuye-agent main(烛夜更名后路径为 `crates/resources/zhuye-sandbox/`);本仓库指针暂留 `34238dd` —— 更名后结构本仓库尚不能消费,指针随外层更名适配一并前移,临时分支已删除 |
| `e4560bc` | 修复批次(D1–D7 收口):会话恢复回放换 submodule 共享 API —— `reader::replay` 升级后支持任意日期桶 + 轮转副本合并 + 文件名/内部 id 双入口,删除本仓库此前维护的全盘扫描 workaround(`resume.rs::collect_session_files`);`JsonlRolloutWriter` 恢复后续写原日期目录的原文件(会话不再被拆成两份);「会话不存在」仍显式报错(共享 API 返回空 Vec,包装层兜底) |
| `7912323` | **未接线 submodule 能力全量接线**(v1.5 / v1.7):`EventMsg::ToolCallArgsDelta` 流式工具参数(live 区渐进预览 + 参数摘要进调用行)、`EventMsg::Routing` 路由事件(failover / 冷却 / `SpecFallback` 跨模型降级)、`reflect_rollout::search_sessions` 跨会话全文搜索(新增 `session search`)、`reflect_config::resolve_model_spec` + `[model_discovery]`(`model = "free"` 运行时解析,配 `routing_policy_with` 传真实 spec)、`reflect_llm::model_catalog` provider 模型清单 + 免费判定(`/model` 无参列出)、`reflect_goal::GoalState` 权威读口(`/goal` 无参显示状态 + `--verify` / `--budget` 选项)、`reflect_config::init_exporter` OTLP 分析遥测、`reflect_config::ConfigWatcher` 配置热重载、`reflect_mcp::curated_catalog` MCP 精选目录(`/mcp` overlay)、`reflect_lsp::DiagnosticRegistry` 诊断(`/lsp`)、`AgentConfig::todos_slot` 待办清单(`/todo`)、`reflect_sandbox::OsSandbox::status_line` 沙箱状态(`/status`)、`reflect_stream::workspace_sync` 工作区克隆(新增 `workspace clone`)、`reflect_discussion::cli::run` 多 agent 讨论(新增 `discussion run`)、`reflect_integration::security_scan` 安全扫描(新增 `security audit`)、`reflect_config::TuiNotificationsSection` 终端通知开关与通道(`[notifications.tui]` 真正生效);同时修 `[sandbox].os_level` 改 `Option<bool>` 带来的编译断点 |
| `d771c9e` | 动态 model 解析接线(`model = "free"` → 启动期经 `reflect_config::resolve_model_spec` 从 provider 清单挑出真实 id,不再拿占位串 `provider/free` 去打请求)+ routing policy 改用 `routing_policy_with(已解析 spec)`(同步版 `routing_policy()` 会让占位串盖掉真实 id)。`SpecSlot.fallbacks` 跨模型降级链由引擎侧接通,TUI 沿用 `EventMsg::Routing` 兜底分支不新增 UI |
| `e4bc182` | 模型级 token 统计（`TokenCountEvent.model` → HUD / `/status` 按模型用量）、`reflect_rollout::session_stats` / `all_model_usage` 接入 `session show` / 新增 `session stats`、`[telemetry].redact` / `max_bytes` / `record_tool_schemas` 真正生效、span 日志读取侧（新增 `traces spans`）、会话文件定位改走共享 API（文件名 thread id 与内部 `session_id` 可能不同） |
| `46dc156` | 提示词分层库接入 + `/prompt` 运行时管理 |
| `3cb791f` | `fetch_update` 弃用告警收口 + CI 编译修复 |
| `8564f66` | 安全边界与正确性收口（沙箱逃逸 / 路径穿越 / Plan mode） |
| `af0483b` | 持久 shell 状态 + `exec --image` 多模态 + `web_fetch` DNS rebinding 防护 |
| `bb2eaeb` | Linux CI 全绿（landlock 降级重试 + 环境自适应测试） |
| `d9b88de` | `/permissions` 弹窗修复 + MCP Prompts 工具接入 |
| `b625b5e` | `/rewind` 弹窗补齐 + clippy 门槛修复 |
| `ce57d48` | 核心审批设置用例改用无副作用 Prompt 探针,消除 Windows 无 OS 沙箱后端时的平台耦合;TUI 公开 API 与行为零变更,仅接线测试修复 |
| `e383206` | 核心文件任务存储的 Windows 文件锁修复:锁文件原以 `append(true)` 打开,Rust std 会刻意剔除 `FILE_WRITE_DATA`,句柄不带 `GENERIC_READ`,而 Windows 的 `LockFileEx` 要求句柄具备读权限 —— 加锁直接返回 `ACCESS_DENIED`(code 5),表现为文件任务后端在 Windows 上完全不可用。改为 `read + write` 打开后跨进程串行化恢复正常;TUI 公开 API 与行为零变更,仅接线上游 bug 修复 |
| `c728da8` | 上游补充:锁文件打开方式显式声明 `truncate(false)`(消除 `clippy::suspicious_open_options`,行为不变);本仓库无适配 |
| `9332aa9` | develop 分支接线(v0.1.8-39,**重叠功能全面改用 submodule**):①`/checkpoint` 的 git 实现收编 —— 删除本地 `git_checkpoint.rs`,改调 `zhuye_git::{create,list,rewind}`(前缀更名 `reflect-checkpoint:`→`zhuye-checkpoint:`、`reflect-cp/`→`zhuye-cp/`,见 COVERAGE.md D15);②分支 / PR 探测收编 —— 删除本地 `branch_summary.rs`(499 行),`zhuye_git::branch` 跑在 `CommandRunnerBridge` 上(本地 `WorkspaceCommandExecutor` app-server 传输边界保留,孤儿规则经借用型 newtype 桥接);③assistant git 指令解析收编 —— 删除本地 `git_action_directives.rs`,改用 `zhuye_git::parse_assistant_markdown`;④home 解析收编 —— 删除 `utils_home_dir.rs`,`find_reflect_home()` 五处消费点改 `zhuye_config::data_dir()` 单一事实源(多 getpwuid 兜底);⑤原子写收编 —— 导出落盘 / 提示词历史 / 主题持久化 / sixel 缓存四处手写 temp+rename 改 `zhuye_fs::write_atomic`(失败保全 + symlink 穿透);⑥协议三事件接线 —— `SideChannelStarted/Finished` 以 Notice 进对话流,`SideChannelStatus` 查询应答丢弃;⑦根 `[workspace.dependencies]` 补 `zhuye-fs`/`zhuye-git` 镜像声明,删除已不存在的 `reflect` crate 声明;新增 side-channel ×3 + 桥接 ×2 Rust 单测 |
| `411e282` | 核心 zhuye-git 命令超时 Windows 整树终止(Job Object,KILL_ON_JOB_CLOSE):孙进程不再逃逸,与 Unix `setpgid`+`kill(-pgid)` 对等;`timed_out_process_cannot_write_after_return` 的 Windows 恒红根因,本仓库无适配 |
| `cd4ad54` | 核心 zhuye-lsp / zhuye-mcp stdio 子进程 `env_clear` 后透传 Windows 运行时候选变量(SystemRoot/windir/ComSpec/PATHEXT 等):缺 SystemRoot 时 node.exe 等 CRT 程序在输出任何字节前崩溃,表现为连接 EOF/Cancelled 而非 spawn 错误;本仓库无适配 |
| `8e72bc4` | 核心 zhuye-shell 会话 kill Windows 整树终止(Job Object)+ env_clear 分支同补运行时变量;`zhuye exec`/常驻 shell 池在 Windows 上 kill 语义与 Unix 对齐;本仓库无适配 |
| `0cca4c6` | 核心 zhuye-telemetry / zhuye-audit 文件锁与追加句柄带读权限:Windows `LockFileEx` 要求 GENERIC_READ/GENERIC_WRITE,纯 append 句柄加锁直接 ACCESS_DENIED(code 5),`activity.rs` 5 个落盘单测的 Windows 恒红根因(此前被 zhuye-git 树杀卡死掩盖,从未跑到该二进制);与 `1c09f6a4→c728da8e` 的 zhuye-task 等三处同一口径的补漏;本仓库无适配 |
| `8abd4ed` | 核心 zhuye-tools safe_write 原子改名切到 FileRenameInformationEx(类 3 拒绝句柄根→Windows code 87,write/edit/plan_write 落盘恒红)+ plan_write 绝对路径按分量判定;本仓库无适配 |
| `770087c` | 指针 883a581 → 770087c:上游 `fix(ci)` 修 `zhuye-core/tests/single_turn.rs:1527` 的 redundant clone(本仓库 `cargo clippy --workspace --all-targets` 因 cargo 自动成员吸收把 submodule 测试目标纳入 lint 面而挂红)+ 覆盖计数刷新;区间内另含 56f8836(审批等待可被回合取消 / 审批落盘 / headless 自动应答)、9aa360f + ce57058(工具×模式×阻塞组合穷举修复与回归测试)、19321c5(Cargo.lock 补录)、4b5f384(docs i18n);协议层(`crates/protocol`)零变更,本仓库无代码适配 |
| `2fb502f` | 指针 770087c → 2fb502f:区间含四项引擎侧新增 —— ① `[[provider_group]]` 模型组(zhuye-config schema + builder,池名 = 组 id,热重载 / failover / `model = "free"` 清单发现自动兼容);② zhuye-llm models.dev 目录(`enrich_model_info` 内嵌快照,未知模型窗口 / 输出上限 / 思考档位自动填充,零接线收益);③ zhuye-skills skills.sh 目录发现(`fetch_index` / `search_entries`,本仓库暂未接线,留待 `/skills` picker 改造);④ zhuye-browser 新 crate(CDP 受控浏览器 + 8 个 browser_* 工具,经 `plugins/browser` 插件安装路径启用,不在本仓库依赖图,根 Cargo.toml 无需镜像)。本仓库代码适配四处:根 `[workspace.dependencies]` 补 `tar`/`flate2` 镜像声明(zhuye-skills 的 skills.sh tarball 解包以 `workspace = true` 继承,跨 workspace 解析落到本根清单,缺声明即 manifest 加载失败)+ `ReflectConfig::active_provider()` 返回类型由 `Option<&'static str>` 改 `Option<String>`(模型组 id 为动态值)→ `/model` 清单路径 `fetch_catalog` / `format_catalog` 调用点改传引用(tui/mod.rs)+ `config_reload::diff_sections` 补 `provider_group` 段比(此前改组配置后热重载 Notice 报「变更段: (无)」;引擎 exec 的 diff_sections 同样漏本段,submodule 只读,TUI 侧先行补上)+ `build_quota_tracker` 遍历 `[[provider_group]]` 显式枚举凭据的 quota 声明(provider 名 = 组 id;组级 api_key wrap 出的 default 凭据 quota 恒 None,无需处理);新增 config_reload ×1 + bootstrap_wiring ×3 Rust 单测;协议层(`crates/protocol`)零变更 |
| `8da90e5` | 指针 2fb502f → 8da90e5(v0.1.15-68):**配置模型 v2 大版本** —— `[[provider_group]]` 更名 `[[model_group]]` 并成为唯一接入单元(固定段 `[anthropic]` / `[openai]` / `[ollama]` 移除,v1 旧配置由 `load` 的迁移逻辑就地转成等价模型组,`[active].provider` → `[active].group`,`[subagent_providers]` → `[subagent_models]`,新增 `config_version` 字段);引擎 exec 的 diff_sections 自本版起已覆盖 `model_group`(2fb502f 行「TUI 先行补」的口径归位)。本仓库代码适配六处:① `build_quota_tracker` 改遍历 `cfg.model_group`(内置三段字段已不存在,组级 base_url 回退逻辑保留,`ProviderGroupSection` → `ModelGroupSection`);② `config_reload::diff_sections` 去 anthropic/openai/ollama 段比、`provider_group` → `model_group`、`subagent_providers` → `subagent_models`,并补 v2 新增段(ask_user_question / prompt_guard / subagents / postgres_session / sse_redis / bridge / voice / dap / acp),保持 TUI 补全版口径;③ bootstrap.rs + tui/mod.rs 共三处 `active_provider()` 改名 `active_group()`(签名不变 `Option<String>`),启动报错文案改「模型组」语汇;④ 根 `[workspace.dependencies]` 的 zhuye-kms 镜像路径跟移(abilities → integrations,上游 `6704a11` 归位);⑤ **门控型 bundled 插件接线**(上游 `7b4230c` 基建):工具注册从手工清单切到单一事实源 `zhuye_tools::register_builtin_tools_with_gates`(TUI 工具面由 17 → 27,补 BackgroundStatus / Worktree ×2 / RequestHumanInput / ToolSearch / NotebookEdit / Brief / GetContextRemaining / Checkpoint / Rewind;web_search / notebook_edit 按 `zhuye_plugin::bundled_tool_gates` 门控注册,`[plugins].disabled_plugins` 显式禁用 = 不进注册表),AskUserQuestion 上限改从 `[ask_user_question]` 段注入,verify-hooks 插件启用 = hooks 注入 `verification` / `test_runner`(build_engine 前),bootstrap_lsp 增 `lsp_plugin_disabled` 参数(`plugins.is_disabled("lsp")`,config 激活语义不变);⑥ 死文件 `cli/models.rs`(未在 cli/mod.rs 声明)同步 `active_group()` 防腐。功能用例:B10 / W10 断言口径改 v2 文案(错误含 `模型组` / `[[model_group]]`),新增 G5(门控默认态零变化)/ G6(`disabled_plugins` 注册门控行为级断言)×2;新增 config_reload ×1(subagent_models 段比)Rust 单测;协议层零变更 |

## 提交 / PR

- commit message 用简洁的 `type: 摘要`（feat / fix / docs / test / chore / refactor）。
- PR 请描述动机、改动点和验证方式（跑了哪些测试 / 手工冒烟步骤）。
- 涉及 submodule 指针变更的 PR 请单独说明。

## 发布前审查

发版前跑一次全量审查，覆盖**编译面 + 行为面 + 覆盖自审**：

```bash
make release-check
```

它依次执行：

1. `./tests/run_all.sh` —— Rust 编译面（`cargo test` / `clippy -D warnings` /
   `fmt` / `doc`）+ 能力覆盖层（真二进制 + pty 的 215 条行为断言 + CLI 参数矩阵）；
2. `tests/tools/coverage_audit.py --strict` —— 确认「声明的 slash 命令」
   全部有行为级用例。

三者任一失败即**不应发版**。功能覆盖矩阵（含已知产品限制、已知缺陷、
未覆盖项的诚实清单）见 [tests/COVERAGE.md](tests/COVERAGE.md)。

> 发布用的是 `./scripts/build.sh --dist`，它只负责构建与打包，**不含**测试。
> 审查与打包是两步：先 `make release-check`，再 `make dist`。

## 报告问题

- 开 issue 时附：操作系统、终端（iTerm/tmux/...）、复现步骤、
  `~/.zhuye/logs/tui-<日期>.log` 的相关片段（注意脱敏）。
- 渲染类问题请附截图或屏幕文本 dump；崩溃类附 panic 全文。

## 行为准则

参与本项目即表示同意 [行为准则](CODE_OF_CONDUCT.md)。
