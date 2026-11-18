# AGENTS.md — zhuye-cli crate

> **何时读我**:改本 crate 的构建面(Cargo / feature / patch)、src 根文件群(bootstrap /
> terminal / viewport 等)、桩层或 `picker/`,或新增 / 移除 src 模块目录前,先读本篇。

## 模块概述

本 crate 是 ZhuYe 的 TUI 表现层(lib `zhuye_cli` + thin binary `zhuye-cli`),引擎
(AgentThread / StateGraph / 工具 / 协议)全部经 path 依赖来自 `zhuye-agent/` submodule。
src/ 约 489 个 `.rs`、18 万行,其中 `tui_core/` 约占 82% —— 真正的"层"在 src 模块
目录,不在 `crates/*`。全局约定与验证矩阵见根 [AGENTS.md](../../AGENTS.md)。

## src 模块速览(改哪个目录读哪篇)

| 目录 / 文件群 | 职责一句话 | 进它读哪篇 |
|---|---|---|
| `src/tui/`(mod.rs + conversion.rs) | 事件循环、按键路由、slash 枚举通道分发 | [src/tui/AGENTS.md](src/tui/AGENTS.md) |
| `src/events.rs` + `src/adapter.rs` | UiEvent / UiState 状态机、协议事件投影(契约与事件循环内聚,归 tui 篇) | [src/tui/AGENTS.md](src/tui/AGENTS.md) |
| `src/tui_core/` | vendored 上游呈现原语(占 src 约 82%) | [src/tui_core/AGENTS.md](src/tui_core/AGENTS.md) |
| `src/history_render.rs` + `src/history_render/` | 历史行渲染、statusline、slash legacy 通道 | [src/history_render/AGENTS.md](src/history_render/AGENTS.md) |
| `src/cli/` + `src/bin/main.rs` | headless 子命令与 clap 路由 | [src/cli/AGENTS.md](src/cli/AGENTS.md) |
| `src/picker/` | overlay 选择器(keymap / theme / model / traces 等 12 个) | 本篇 §picker |
| 桩层:`protocol_compat/`、`app_server_protocol/`、`config_compat/`、`app_server_client.rs` | 上游 crate 类型的本地重声明,供 tui_core 消费 | 本篇 §桩层 |
| 其余 src 根文件(`bootstrap.rs`、`bootstrap_wiring.rs`、`terminal.rs`、`viewport.rs`、`user_messages.rs`、`transcript_pager.rs`、`tasks_pager.rs` 等) | 装配、终端基础设施、滚动回放 | 本篇 |

## 本地契约 / 同步点

1. **根 `[workspace.dependencies]` 必须完整声明**:嵌套 workspace 下核心 crate 的
   `workspace = true` 继承会解析到本仓库根清单(根篇关键约束 1)。升级 submodule 或
   核心 crate 新增依赖时同步补声明,缺一条即 manifest 加载失败。
2. **`[patch.crates-io]` 固定 nornagon fork rev**(根 `Cargo.toml:183-185`,crossterm
   `87db8bfa…` / ratatui `9b2ad12…`):fork 暴露官方版没有的 unstable features,
   `tui_core/custom_terminal.rs` 与 `insert_history.rs` 深度依赖,不得改回 crates.io 官方版。
3. **feature 面只有 `tui-upstream-tests`**(声明于 `Cargo.toml:15-20`):
   门控语义、门控点分布与新增 vendored 测试的 gate 规则,单点见 tui_core 篇。
4. **`.cargo/config.toml` 的 `INSTA_WORKSPACE_ROOT` 修复勿删**:insta 在嵌套 workspace
   下会误算 snapshot 路径(产生双层 `zhuye-agent/zhuye-agent/...`);快照资产分布见 tui_core 篇。
5. **TUI 主循环必须 `current_thread` runtime**(`bootstrap.rs:70-75`):fork 版 crossterm
   的 `event::poll/read` 锁绑定调用线程,多线程下 poll 恒 false。唯一例外:`discussion`
   子命令必须多线程 `Runtime::new()`(跨 task 等待,current_thread 死锁,例外细节见
   cli 篇)。新增入口时按此选 runtime,不要把 TUI 路径改成多线程。
6. **`unsafe` 边界**:`bootstrap.rs:110-119` 的 `set_var` 仅限 bootstrap 单线程期
   (SAFETY 论证在注释内),不得在多线程期复制该写法;其余 unsafe 集中在 tui_core(见该篇)。
7. **PlanWrite 三件套**(`bootstrap.rs:167-174`):`PlanWriteTool` 必须与
   Enter/ExitPlanMode 一起注册(ALWAYS_ON_TOOLS 含 PlanWrite,缺注册即 "tool not found");
   TUI 装配对齐 headless `zhuye-exec/src/bootstrap.rs` 参照物(`bootstrap_wiring.rs` 头注释)。
8. **导出 / 落盘统一走 `zhuye_fs::write_atomic`**,不要手写 temp+rename。

## 桩层

`protocol_compat`(约 1.6K 行)/ `app_server_protocol`(约 2.9K)/ `config_compat`
(约 1.3K)/ `app_server_client.rs` 是给 `tui_core/` vendored 代码用的本地类型桩,
**覆盖范围由 tui_core 内的 `crate::<桩层>` 引用驱动**(`protocol_compat/mod.rs:1-12`):

- tui_core 引用为大头(抽样约:protocol_compat 280 处 / app_server_protocol 189 /
  config_compat 72),增删引用时桩层类型随之增删,桩层不追求与上游一一对应;
- 不要把桩层升级成"真实现":引擎能力一律走 submodule 公开 API;
- 桩层类型改动只影响 tui_core 编译面,跑 `make check` 即可暴露。

## picker/

overlay 选择器(checkpoint / copy-history / fork-rewind / image / keymap / mcp /
plugin / skills-hub / statusline / theme / traces 等),`picker/mod.rs:1-4`:由
`tui/mod.rs::draw` 叠加调用,依赖 tui_core + adapter / events。各 picker 的
**标题字面量**是 `tests/tools/coverage_audit.py` 的解析输入之一,改标题前先看
[tests/AGENTS.md](../../tests/AGENTS.md)。

## 禁止事项

- 不要把核心 crate 列入 `workspace.members`,也不要删根清单里的 zhuye-* 镜像声明。
- 不要在 TUI 路径引入多线程 runtime;不要绕过 SAFETY 注释新增 `unsafe` 环境变量写。
- 不要在桩层复刻引擎逻辑;不要让 `src/cli/` 依赖 `tui_core`(headless 不碰渲染层)。
- 不要新增 src 模块目录而不建篇 / 不更新本篇速览表与根导航表。

## 深入阅读

- 根 [AGENTS.md](../../AGENTS.md)(全局约定 / 验证矩阵)· [CONTRIBUTING.md](../../CONTRIBUTING.md)(submodule 工作流)
- [NOTICE](../../NOTICE)(fork 与 vendored 来源的许可证归属)
