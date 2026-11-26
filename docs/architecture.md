# 架构详解

<p align="center">  <b>中文</b> · <a href="architecture.en.md">English</a>  </p>

> ZhuYe-CLI 的架构参考:仓库定位与引擎关系、表现层模块分层、事件循环与
> UiState 状态机、协议投影边界,以及跨 workspace 的依赖约束。README 讲用法,
> 本文面向需要理解内部实现或二次开发的读者。配置参考见
> [configuration.md](configuration.md);开发与测试流程见
> [development.md](development.md)。

## 1. 仓库定位:表现层 + 引擎 submodule

ZhuYe-CLI 是 ZhuYe 的终端交互式客户端(ratatui + crossterm),面向终端
AI Agent 工作流。本仓库是**单 crate workspace**:仅 `crates/zhuye-cli`
(库 `zhuye_cli` + 薄二进制 `zhuye-cli`)。

核心引擎(AgentThread / StateGraph / 工具 / 协议)由 `zhuye-agent/`
**git submodule** 提供,本 crate 只负责表现层:事件循环、布局、渲染、
协议事件适配。

```
zhuye-agent (核心引擎,submodule 提供方)
  └─ crates/{protocol,abilities,resources,orchestration,integrations,runtime}
  └─ zhuye-core (AgentThread + StateGraph)

ZhuYe-CLI (本仓库)  ──submodule──► zhuye-agent
  └─ crates/zhuye-cli (ratatui 交互界面 + 事件循环)
  └─ zhuye-cli 二进制 (终端入口)
```

无子命令启动即进入 TUI;`session` / `traces` 等子命令走 headless 路径,
执行完即退出。

## 2. 模块分层(依赖方向自上而下)

| 模块 | 职责 |
|------|------|
| `src/bin/main.rs` | 薄二进制:clap 路由。无子命令 → TUI;`session`/`traces`/`workspace`/`discussion`/`security` → headless 执行完退出 |
| `bootstrap.rs` | 加载 config、构建 ModelRegistry / provider / ToolRegistry / AgentThread,进入 TUI |
| `tui/` + `events.rs` + `adapter.rs` | **本地**事件循环、布局、`UiEvent`/`UiState` 状态机、协议事件适配边界 |
| `tui_core/` | 自包含呈现原语(自定义终端后端、流式历史插入、CJK 换行、composer 等) |
| `history_render/`(根 `history_render.rs` + 目录) | 历史行渲染、statusline、slash legacy 通道 |
| `cli/`(session / traces / discussion / security / workspace;models.rs 未接线) | headless 子命令实现(调 zhuye-rollout / zhuye-telemetry / zhuye-discussion / zhuye-integration) |
| `views/`、`term/`、`utils/`、`crate_stubs/`、`compat/` | 原 src 根平铺文件按功能归组:人读渲染/overlay 视图、终端基础设施、辅助函数、`zhuye_*` 引擎桩、兼容垫片(旧路径经 lib.rs 垫片兼容) |

依赖方向严格单向:`tui` → `tui_core` / `adapter` / `events` /
`history_render` / `terminal`,任何模块不得反向引用 `crate::tui`。

## 3. 事件循环与 UiState 状态机

TUI 的事件循环是**本地单线程**的:

- `tui/mod.rs::run_async` 是事件循环入口;`handle_event` 消化终端事件,
  `handle_key` 做按键路由,`draw` / `draw_fullscreen` 负责渲染;
- `events.rs` 定义 `UiState`(共享可变状态)、`LoopState`、审批挂起态
  (`PendingApproval` / `PlanApprovalState`);
- `adapter.rs` 定义 `UiEvent` / `UiHistoryItem` 与状态归约器 `apply_event`
  —— 引擎事件在这里被归约为 UI 状态变更 + 需要 flush 进滚动区的行;
- 后台任务(`background_tasks`)在主循环 spawn / abort,abort 时机与
  轮次中断语义绑定。

**bootstrap 必须用 `current_thread` runtime**:fork 版 crossterm 的
`event::poll/read` 绑定调用线程内部锁,多线程 runtime 下 poll 会因锁竞争
恒 false(见 `bootstrap.rs` 注释)。

## 4. 协议投影:Submission / EventMsg 的消费边界

TUI 与引擎之间只隔一层协议:

- 用户输入经 composer 校验后作为 `Submission` 提交给 `AgentThread`;
- 引擎侧事件(`EventMsg`)统一在 `tui/conversion.rs` 转换为 UI 事件 ——
  这是**唯一**的转换边界,转换逻辑不允许散落在 `handle_key` / `draw` 里;
- 对话轮次落盘的 assistant 消息是**最后一轮的回复**:引擎一次用户提交
  会多次调用模型,流式中间态不入盘。

wire 协议(Submission / Op / EventMsg)的演进属于引擎仓库,本仓库只做
消费端适配。

## 5. 跨 workspace 依赖约束

- **Cargo 不支持嵌套 workspace**:核心 crate 不列入本仓库
  `workspace.members`,只用 `path` 依赖。核心 crate 大量
  `workspace = true` 继承依赖会解析到**本仓库**根 `[workspace.dependencies]`,
  因此该段必须完整声明核心引用的全部 zhuye-* 与外部依赖 —— 升级 submodule
  或新增核心 crate 依赖时,同步补声明。
- **ratatui/crossterm 是 nornagon fork**(根 `Cargo.toml`
  `[patch.crates-io]` 固定 rev),依赖官方版没有的 unstable feature
  (`scrolling-regions`、`unstable-backend-writer`、`rendered-line-info`、
  `WidgetRef`、`event-stream`、`bracketed-paste`),不能换回 crates.io
  官方版。
- `tui_core/` 为本仓库自包含的呈现原语,行为以本仓库代码为准;第三方
  来源与许可证见 [NOTICE](../NOTICE)。

引擎侧的分层架构(六层 crate 组织、StateGraph 状态转移)见 submodule 的
[zhuye-agent/docs/architecture.md](../zhuye-agent/docs/architecture.md)。

## 6. 构建 profile 策略

- `[profile.release]`:thin LTO + `codegen-units = 1` + `strip = "symbols"`,
  供发布分发。
- `[profile.release-fast]`:无 LTO、`opt-level = 2`、`incremental = true`,
  供本地快速迭代(`scripts/build.sh --fast`);产物落在
  `target/release-fast/`,增量目录不会被 cargo 自动回收 —— 由 `make gc` 清理。
- `[profile.dev]`:刻意关闭 `incremental` —— cargo 不会自动 GC
  `target/<profile>/incremental/`,开启增量会让磁盘占用无界增长;代价是
  首次构建略慢。
- `[profile.test]`:`debug = 1`(仅行号)+ `codegen-units = 256`,
  加速测试产物的编译与链接。

清理面的完整语义(含交叉编译产物根 `target/<triple>/<profile>/`)见根
`Makefile` 的 `gc` / `gc-deep` 目标。

## 7. 深入阅读

- [AGENTS.md](../AGENTS.md) —— AI 助手工作契约:模块分层、关键约束、开发流程检查表
- [docs/configuration.md](configuration.md) —— 数据目录、启动参数、环境变量
- [docs/development.md](development.md) —— 构建、测试分层、验证矩阵
- [zhuye-agent/docs/architecture.md](../zhuye-agent/docs/architecture.md) —— 引擎侧架构参考(外链,submodule)
