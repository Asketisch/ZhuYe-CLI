# AGENTS.md

> 本文件是本仓库的 AI 助手说明(原 `CLAUDE.md`,2026-10 起改名)。
> submodule 侧的同名文件见 `zhuye-agent/AGENTS.md`,两者结构对标。

## 项目概览

ZhuYe-CLI — ZhuYe 的终端交互式客户端(ratatui + crossterm),面向终端 AI Agent 工作流。

- 单 crate workspace:仅 `crates/zhuye-cli`(lib `zhuye_cli` + thin binary `zhuye-cli`)
- 核心引擎(AgentThread / StateGraph / 工具 / 协议)由 `zhuye-agent/` **git submodule** 提供,
  本 crate 只负责表现层:事件循环、布局、渲染、协议事件适配
- Rust stable,edition 2024,MSRV 1.85(`rust-toolchain.toml` 锁定)
- 用户数据目录:`~/.zhuye/`(config.toml / sessions/ / traces/);Plan 草稿写 `<workspace>/.zhuye/plan/`

## AGENTS.md 体系导航

本仓库的目录级契约按「渐进式披露」分层:根篇(本篇)恒在上下文只做导航与全局约定,
具体目录的本地契约在就近的分层篇里,用户向系统说明在 `docs/`(中英双语)。
**修改任何目录前,先读该目录(或其所在 crate 篇)的 AGENTS.md。**

| 想了解什么 | 读哪个 AGENTS.md |
|---|---|
| slash 命令双通道(legacy 白名单与结局;枚举面在 tui_core、分发在 tui、门禁在 tests,见下方各行) | [src/history_render/](crates/zhuye-cli/src/history_render/AGENTS.md) |
| 事件循环 / 按键路由 / UiState / 协议投影 | [src/tui/](crates/zhuye-cli/src/tui/AGENTS.md) |
| 呈现原语(vendored 上游 / 换行 / composer / 快照) | [src/tui_core/](crates/zhuye-cli/src/tui_core/AGENTS.md) |
| headless 子命令(session / traces / clap 路由) | [src/cli/](crates/zhuye-cli/src/cli/AGENTS.md) |
| 构建面 / 依赖 / feature / bootstrap / 桩层 / picker | [crates/zhuye-cli/](crates/zhuye-cli/AGENTS.md) |
| 测试分层 / 组字母 / coverage-audit / CI 门禁 | [tests/](tests/AGENTS.md) |
| 构建 / 发版 / 安装脚本 | [scripts/](scripts/AGENTS.md) |
| 架构与系统说明(用户向,中英双语) | [docs/architecture.md](docs/architecture.md) · [README 文档表](README.md)(外链) |
| 核心引擎(submodule,只读消费) | [zhuye-agent/AGENTS.md](zhuye-agent/AGENTS.md)(外链,展开见下方「submodule 侧索引」) |
| 贡献流程 / submodule 升级 | [CONTRIBUTING.md](CONTRIBUTING.md)(外链) |
| 第三方来源与许可证 | [NOTICE](NOTICE)(外链) |

**submodule 侧索引(只读)**:`zhuye-agent/` 内已落地同构的分层 AGENTS.md 体系
(6 层篇 + 8 crate 篇 + 3 顶层目录篇)。本仓库只读消费、**勿在其内修改**;
升级 submodule 指针时,按此表定位受影响的引擎契约篇再核对消费点:

| 想了解什么(引擎侧) | 读 `zhuye-agent/` 内哪篇 |
|---|---|
| 协议契约:Op / EventMsg / Item / Submission | [crates/protocol/](zhuye-agent/crates/protocol/AGENTS.md) · [zhuye-protocol/](zhuye-agent/crates/protocol/zhuye-protocol/AGENTS.md) |
| 能力原语:模型 / 工具 / hook | [crates/abilities/](zhuye-agent/crates/abilities/AGENTS.md) · [zhuye-llm/](zhuye-agent/crates/abilities/zhuye-llm/AGENTS.md) · [zhuye-tools/](zhuye-agent/crates/abilities/zhuye-tools/AGENTS.md) · [zhuye-hooks/](zhuye-agent/crates/abilities/zhuye-hooks/AGENTS.md) |
| 资源层:配置 / 权限 / 插件 / 持久化 | [crates/resources/](zhuye-agent/crates/resources/AGENTS.md) · [zhuye-plugin/](zhuye-agent/crates/resources/zhuye-plugin/AGENTS.md) |
| 多 agent 编排:子代理 / 讨论 / 任务 | [crates/orchestration/](zhuye-agent/crates/orchestration/AGENTS.md) |
| 外部集成:MCP / LSP / git | [crates/integrations/](zhuye-agent/crates/integrations/AGENTS.md) |
| 运行时与入口:AgentThread / StateGraph / headless / 引擎 CLI | [crates/runtime/](zhuye-agent/crates/runtime/AGENTS.md) · [zhuye-core/](zhuye-agent/crates/runtime/zhuye-core/AGENTS.md) · [zhuye-exec/](zhuye-agent/crates/runtime/zhuye-exec/AGENTS.md) · [zhuye-cli/](zhuye-agent/crates/runtime/zhuye-cli/AGENTS.md) |
| 引擎侧脚本 / 双 SDK / 测试分层 | [scripts/](zhuye-agent/scripts/AGENTS.md) · [sdks/](zhuye-agent/sdks/AGENTS.md) · [tests/](zhuye-agent/tests/AGENTS.md) |

## 常用命令

```bash
make build                      # cargo build --release -p zhuye-cli
make run                        # 构建并启动 TUI
make check / make check-fast C=zhuye-cli   # 类型检查(全 workspace / 单 crate)
make test                       # cargo test --workspace
make test-fast                  # cargo nextest(需已安装 cargo-nextest)
make test-suite                 # ./tests/run_all.sh:全功能套件(能力覆盖层+编译面+覆盖自审)
make test-suite-fast            # ./tests/run_all.sh functional:只跑能力覆盖层(跳过 cargo 编译面)
make test-suite-live            # ./tests/run_all.sh live:live 端到端(需 API key,手动跑)
make coverage-audit             # 覆盖自审:slash 命令 × 测试矩阵的差集核对(有缺口即失败)
make release-check              # 发版前全量审查(编译面 + 能力覆盖层 + 覆盖自审)
make dist                       # ./scripts/build.sh --dist:构建+打包当前平台分发包到 dist/
make dist-dry                   # 预览 dist 打包命令
make install                    # 装到 /usr/local/bin(或 --local → ~/.local/bin)
make gc                         # 清产物根下全部 profile 的 incremental 缓存(不触发重编)
make gc-deep                    # 深清理:再清 deps/.fingerprint/examples(下次构建接近冷构建)

# 全功能测试套件(tests/ 目录,分层见 tests/README.md,覆盖矩阵见 tests/COVERAGE.md)
./tests/run_all.sh              # 离线全量:能力覆盖层 + Rust 编译面 + 覆盖自审
./tests/run_all.sh functional   # 只跑能力覆盖层(真二进制 + pty 行为断言 + CLI 参数矩阵)
./tests/run_all.sh audit        # 只跑覆盖自审
./tests/run_all.sh live         # live 端到端(真实 LLM + 真实工具,需 API key,手动跑)
```

## 架构

### 本 crate 内部分层(依赖方向自上而下)

| 模块 | 职责 |
|------|------|
| `src/bin/main.rs` | thin binary:clap 路由。无子命令 → TUI;`session`/`traces` → headless 执行完退出 |
| `bootstrap.rs` | 加载 config、构建 ModelRegistry / provider / ToolRegistry / AgentThread,进入 TUI |
| `tui/` + `events.rs` + `adapter.rs` | **本地**事件循环、布局、`UiEvent`/`UiState` 状态机、协议事件适配边界 |
| `tui_core/` | 自包含呈现原语(自定义终端后端、流式历史插入、CJK 换行、composer 等) |
| `history_render/`、`transcript_pager.rs`、`tasks_pager.rs` | 历史行渲染、滚动回放、任务面板 |
| `cli/{session,traces}.rs` | headless 子命令实现(调 zhuye-rollout / zhuye-telemetry) |

### 核心引擎(submodule,只读消费,勿改)

`zhuye-agent/crates/{protocol,abilities,resources,orchestration,integrations,runtime}(烛夜更名后 crate 名为 `zhuye-*`)` 六层。
本 crate 直接依赖的关键 crate:zhuye-core(AgentThread)、zhuye-llm(ModelRegistry)、
zhuye-tools(内置工具)、zhuye-config、zhuye-rollout、zhuye-telemetry、
zhuye-task/subagent、zhuye-protocol。

## 关键约束(改代码前必读)

1. **Cargo 不支持嵌套 workspace**:核心 crate **不**列入本仓库 `workspace.members`,
   只用 `path` 依赖(`zhuye-core = { path = "zhuye-agent/crates/runtime/zhuye-core" }`)。
   核心 crate 大量 `workspace = true` 继承依赖会解析到**本仓库**根 `[workspace.dependencies]`,
   因此该段必须**完整声明**核心引用的全部 zhuye-* 与外部依赖 —— 升级 submodule 或新增
   核心 crate 依赖时,同步补声明,否则跨 workspace 解析失败。详见 [CONTRIBUTING.md](CONTRIBUTING.md) 的「submodule 工作流」小节。
2. **ratatui/crossterm 是 nornagon fork**(根 `Cargo.toml` `[patch.crates-io]` 固定 rev),
   依赖官方版没有的 unstable feature(`scrolling-regions`、`unstable-backend-writer`、
   `rendered-line-info`、`WidgetRef`、`event-stream`、`bracketed-paste`)。不要改回 crates.io 官方版。
3. **bootstrap 必须用 `current_thread` runtime**:fork 版 crossterm 的 `event::poll/read`
   绑定调用线程内部锁,多线程 runtime 下 poll 会因锁竞争恒 false。见 `bootstrap.rs` 注释。
4. `.cargo/config.toml` 设 `INSTA_WORKSPACE_ROOT=""` 是 insta 在嵌套 workspace 下的路径修复,勿删。
5. `tui_core/` 为本仓库自包含的呈现原语,行为以本仓库代码为准;第三方来源与
   许可证见 [NOTICE](NOTICE)。
6. vendored 上游测试 fixtures 由 feature `tui-upstream-tests` gate,默认关闭
   (`cargo test -p zhuye-cli --features tui-upstream-tests` 才跑)。
7. Plan mode 写盘:PlanWrite 工具把 plan markdown 写到 `<workspace>/.zhuye/plan/<name>.md`,
   ExitPlanMode 读取;`PlanWriteTool` 必须与 Enter/ExitPlanMode 一起注册(见 `bootstrap.rs`)。
8. **slash 命令有两条分发通道**,行为不一致,改命令时两条都要看:
   - `history_render::dispatch_slash`(legacy 通道)—— 命令名命中
     `is_legacy_passthrough_slash` 白名单时走这条;
   - `tui/mod.rs::dispatch_slash_command`(枚举通道)—— 其余可见的
     `SlashCommand` 走这条,**只实现了部分命令**,其余落到
     「此命令在当前构建中尚未接入分发」兜底。
   全量矩阵见 [tests/COVERAGE.md](tests/COVERAGE.md) §S 组;
   **新增 slash 命令必须同时补 `tests/functional/test_slash.py` 的 MATRIX**,
   否则 `make coverage-audit` 会失败。
9. **对话轮次落盘的 assistant 消息是最后一轮的回复**:引擎一次用户提交会
   多次调用模型(mock provider 每次调用消费脚本一行),流式中间态不入盘。

## 开发流程检查表(改任何代码前必读)

> 本节结构与 submodule `zhuye-agent/AGENTS.md` 的同名章节对标。
> **新功能或修改必须与测试脚本一起提交** —— 不接受「先合功能、后补测试」,
> 也不接受只有「退出码 0 / 不崩」这类伪断言。

**新功能**或 **bug 修复**提交前,必须逐条走完:

1. **能力覆盖脚本同步** —— 判断本次改动是否需要新增 / 更新能力覆盖用例:
   - 改了 TUI 对外可见的行为(渲染文本、按键路由、弹层、slash 命令、
     对话轮次、会话落盘、CLI 输出)
     → 必须在 `tests/functional/` 有对应用例,断言到**行为级**
     (屏幕可见文本 / 落盘文件内容 / 进程退出码),
     **不接受**「退出码 0」或「不抛错」作为能力断言;
   - 已有相关用例 → 检查是否需要**更新**(断言口径变了 / 新增字段 / 语义变化);
   - 能力覆盖用例**只写在 `tests/functional/`** —— 不要在 `tests/rust/*.sh`
     里重复写行为断言,那里只管 CLI **参数面**;
   - 只改内部纯逻辑(渲染辅助函数、CJK 换行、状态机转换)
     → 补 Rust 单测(`crates/zhuye-cli/src/**`),不必硬凑 pty 用例。
2. **按改动面选测试落点** —— 能力覆盖层按面分组,新用例进对应组:

   | 改动面 | 用例落点 |
   |---|---|
   | 启动 / 渲染锚点 / 退出生命周期 / 全屏模式 | `tests/functional/test_boot.py`(B 组) |
   | slash 命令面 | `tests/functional/test_slash.py` 的 `MATRIX`(S 组) |
   | 弹层 / 浮层 / 选择器 | `tests/functional/test_overlay.py`(O 组) |
   | 对话轮次 / 流式 / 工具调用 / 统计 | `tests/functional/test_turn.py`(T 组) |
   | 会话持久化 / 恢复 / fork / plan 落盘 | `tests/functional/test_session.py`(P 组) |
   | headless CLI 语义 | `tests/functional/test_cli.py`(C 组) |
   | 键盘交互 | `tests/functional/test_keys.py`(K 组) |
   | 渲染与视觉 | `tests/functional/test_render.py`(V 组) |
   | 审批 / AskUser / 容错 / 信号 | `tests/functional/test_flow.py`(W 组) |
   | 插件面 | `tests/functional/test_plugin.py`(G 组) |
   | CLI 参数矩阵 | `tests/rust/cli_core.sh`、`tests/rust/cli_modules.sh` |
   | 内部纯逻辑 | `crates/zhuye-cli/src/**` 的 Rust 单测 |

   分层判据与用例规模见 [tests/README.md](tests/README.md);
   「能力 × 覆盖层」映射见 [tests/COVERAGE.md](tests/COVERAGE.md)。
3. **slash 命令双通道 + 覆盖门禁** —— 新增 / 改名 / 改结局的 slash 命令:
   - 两条分发通道都要看:`history_render::dispatch_slash`(legacy 白名单)
     与 `tui/mod.rs::dispatch_slash_command`(枚举通道);
   - `tests/functional/test_slash.py` 的 `MATRIX` 必须补一行,写明结局类别
     (`wired` / `placeholder` / `unrecognized` / `exit`)与期望屏幕文本;
   - `make coverage-audit`(`tests/tools/coverage_audit.py --strict`)会做
     「Rust 里声明的命令 × 矩阵覆盖」的**机械差集**,漏一条直接失败。
4. **覆盖矩阵同步** —— `tests/COVERAGE.md` 随改动更新:
   新增能力补「能力 × 覆盖层」表的一行;移除能力移到「未覆盖项」并写明原因;
   发现的缺陷记入「已知缺陷」。
5. **AGENTS.md 分层索引同步** —— 修改或新增代码时,对应目录的 AGENTS.md
   (模块篇、crate 篇、根导航表)必须同步更新;契约 / 同步点 / 禁止事项
   变化即改对应篇,导航目标增删即改根表。按下表定位:

   | 代码改动 | 同步更新 |
   |---|---|
   | 新增 / 移除 src 模块目录 | 新建 / 删除该篇 + crate 篇速览表 + 根导航表 |
   | slash 命令(枚举 / 白名单 / 分发结局) | `src/tui_core/` + `src/tui/` + `src/history_render/` + `tests/`(S 组) |
   | 事件循环 / 按键 / UiState / 协议投影 | `src/tui/` |
   | 呈现原语 / 换行 / 快照 / vendored 边界 | `src/tui_core/` |
   | headless 子命令 / 参数面 | `src/cli/` |
   | 依赖 / feature / bootstrap 装配 / 桩层 / picker | `crates/zhuye-cli/` |
   | 测试分层 / 组结构 / 覆盖矩阵 / 套件入口 | `tests/` |
   | 脚本新增 / 移除 / 行为变更 | `scripts/` 速览表 |
   | 用户可见文档(README / docs/ / SECURITY 等中文版) | 同步更新对应 `.en.md`(不在 CI 门禁,必须同步) |
   | 升级 submodule 指针 / 引擎契约变化 | 按上方「submodule 侧索引」读对应篇核对消费点;本仓库适配面变化则同步 `crates/zhuye-cli/` 篇 |

   拿不准改哪篇时,至少更新 crate 篇与根导航表,确保索引不失真。
6. **验证** —— 按下方「验证矩阵」跑对应命令,全绿再提交。
7. **注释与提交信息** —— 注释一律中文;commit 用 `type(scope): 描述`
   conventional 风格(参考 git log)。

## 验证矩阵(按改动范围选跑)

| 改动范围 | 必跑命令 |
|---|---|
| 任何改动 | `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings` |
| Rust 代码 / crate | `cargo test --workspace` |
| TUI 行为(渲染 / 按键 / 弹层 / 轮次 / 落盘 / CLI 语义) | `./tests/run_all.sh functional` |
| slash 命令 | `./tests/run_all.sh functional` + `make coverage-audit` |
| CLI 子命令参数面 | `./tests/run_all.sh rust` |
| 仅文档 / 注释 | 无需跑测试,自查链接与命令是否仍有效 |
| 发版前 | `make release-check` |

> 全程离线:`ZHUYE_MODEL=mock/mock-1` + 隔离 HOME(mktemp),
> 不污染真实 `~/.zhuye`;缺 pytest / pyte 时能力覆盖层判基础设施错误。
> 首次运行需 `pip install pytest pyte`(Node 不需要)。

## 约定

- **强制:注释一律使用中文** —— 包括行注释(`//`)与块注释(`/* */`)、
  文档注释(`///`、`//!`,crate 模块文档、结构体/函数/字段文档),提交信息与
  PR 描述同样使用中文(参考 git log)。第三方/许可证声明保留原文;
  **用户可见文案(UI 文本、错误提示、日志)不在此限制内**,按现有代码风格。
- **用户可见文档中英双语**:README / docs/ / SECURITY / CONTRIBUTING /
  CODE_OF_CONDUCT 为分文件成对(`x.md` + `x.en.md`,顶部语言切换条互指;
  切换条位于 `<p align="center">` HTML 块内,链接须用 HTML `<a href>`、
  强调须用 `<b>` —— HTML 块内的 markdown 语法(`[text](url)` / `**bold**`)
  GitHub 不解析、会原样显示);改了中文版须同步更新对应 `.en.md` ——
  英文版不在 CI 门禁,但必须一起维护,不允许长期漂移(与 submodule 侧治理规则一致)。
- AGENTS.md 体系与 tests 内部文档保持纯中文,不做双语。

## Submodule 维护

- 升级:`git submodule update --remote zhuye-agent && git add zhuye-agent && git commit`
- 指针变更必须与代码适配一起提交;接线记录追加到 CONTRIBUTING.md「submodule 工作流」段。
- 完整流程(分支切换、联调)见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 第三方来源

ratatui/crossterm fork 等第三方来源与许可证统一见 [NOTICE](NOTICE)。
