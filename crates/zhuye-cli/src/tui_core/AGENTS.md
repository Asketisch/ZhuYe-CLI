# AGENTS.md — tui_core/(自包含呈现原语)

> **何时读我**:改任何呈现原语 —— 换行 / composer / 终端后端 / 历史单元 / markdown
> 渲染 / 快照,或触碰 vendored 上游移植代码 —— 之前,先读本篇。

## 模块概述

本目录约 15 万行、占 src 约 82%,是本仓库自包含的呈现原语集合:大量代码
**vendored 自上游 TUI**(ChatWidget / bottom_pane / history_cell 等),行为以
本仓库代码为准,第三方来源与许可证见 [NOTICE](../../../../NOTICE)。
真实依赖边界:**不引用 `crate::tui`**,但依赖四个桩层(契约见
[crate 篇](../../AGENTS.md) §桩层)与 slash 白名单回环(契约单点在
[history_render 篇](../history_render/AGENTS.md))。

## 二级目录速览

| 目录 | 职责一句话 |
|---|---|
| `bottom_pane/`(约 5.4 万行) | composer 输入区(textarea / slash 输入 / vim / 附件)与各类 footer |
| `chatwidget/`(约 2.3 万行) | 上游 ChatWidget 全量移植,含 embed 模式 slash 分发(`slash_dispatch.rs`) |
| `history_cell/` | 上游历史单元与 markdown 渲染缓存 |
| `markdown_render/` / `diff_render/` | markdown 渲染器 / diff 视图(各自带快照) |
| `streaming/` | 流式管线(StreamState / controller / chunking / commit_tick),队列顺序不变量 |
| `keymap.rs` + `keymap_setup/` | 键位表与配置向导 |
| `pager_overlay/` / `onboarding/` / `pets/` / `inline_visualization/` | 浮层分页 / 首跑引导 / 宠物 / 内联可视化 |
| `ide_context/` | IDE 桥接(Windows named pipe / Unix socket,unsafe 集中区) |
| 根文件(`slash_command.rs` / `custom_terminal.rs` / `insert_history.rs` / `live_wrap.rs` / `test_backend.rs` 等) | 见下表 |

## 核心文件导航

| 文件 | 职责 |
|---|---|
| `slash_command.rs` | `SlashCommand` 枚举(60 个变体,以本文件为准)—— composer 弹窗与枚举通道的命令面 |
| `custom_terminal.rs` | 自定义终端后端,文件头声明衍生自 `ratatui::Terminal`(MIT) |
| `insert_history.rs` | 定稿行以转义序列写进**原生 scrollback**(`HistoryLineWrapPolicy` :43) |
| `live_wrap.rs` + `wrapping/` + `width.rs` | CJK / emoji 宽度感知换行与宽度护栏 |
| `bottom_pane/chat_composer.rs`(约 4.6K 行) | composer 主体,含 `slash_input` / vim / 附件子模块 |
| `public_widgets/composer_input.rs` | `ComposerAction`(:21):供 crate 外(tui / events)复用的最小 composer 接口 |
| `test_backend.rs` | 测试后端,仅 `#[cfg(test)]` |

## 本地契约 / 同步点

1. **`SlashCommand` 顺序即弹窗展示顺序,禁止按字母排序**(`slash_command.rs:13-14`
   注释);新增 / 改名变体走根篇检查表 3(补 MATRIX,过 `make coverage-audit`)。
2. **embed 通道**:`chatwidget/slash_dispatch.rs` 是独立内嵌分发
   (`SlashCommandDispatchSource` :24),`/rewind` `:280`、`/prompt` `:535` 为
   未接入文案 —— 改 embed 命令结局同步 D / S 组断言。
3. **vendored 上游测试必须 gate 在 `tui-upstream-tests` feature 后**(默认关闭,
   `#[cfg(all(test, feature = "tui-upstream-tests"))]`,门控点约 40 处):新移植的
   上游测试文件整文件 gate(示例 `bottom_pane/list_selection_view/tests.rs:1`),
   不得混进默认 `cargo test` 编译面。
4. **fork unstable features 消费面**:`custom_terminal.rs` / `insert_history.rs`
   深度使用 scrolling-regions / unstable-backend-writer / rendered-line-info /
   WidgetRef —— 换官方版 ratatui/crossterm 直接编译失败,rev 锁定见 crate 篇。
5. **CJK / URL 换行不变量**:`live_wrap` 的 RowBuilder、`wrapping/mixed_url`
   (CJK 窄行不死循环)、`diff_render/wrap`(CJK / 制表符死循环防御)、`width.rs`
   护栏 —— 改换行逻辑必须跑 V 组(宽字符 / 折行)与对应 Rust 单测。
6. **快照资产**:insta 快照全部在本目录树内(各模块 `snapshots/` 子目录,约 146 个
   `.snap`)。**`bottom_pane/snapshots/` 与子模块自己的 `snapshots/` 存在同名双副本**,
   是 `INSTA_WORKSPACE_ROOT` 路径迁移的残留 —— 不要手工单删一侧,先跑快照确认
   insta 实际读写的落点再清理。
7. **unsafe 集中区**:`clipboard_copy.rs`(macOS pasteboard)、`pets/image_protocol.rs`
   (env)、`ide_context/`(Win32 named pipe 约 20 处 / Unix socket `SO_PEERCRED`)——
   `ide_context` 新增 import 必须同步 `crates/zhuye-cli/Cargo.toml:177-186` 的
   windows-sys feature 面(注释明说「一一对应」)。
8. `history_cell/markdown_render_cache.rs:37` 对中毒锁 `unwrap_or_else(PoisonError::into_inner)`
   恢复 —— 改缓存结构时保持该恢复语义。

## 禁止事项

- 不改回 crates.io 官方版 ratatui / crossterm;不删除 unstable feature 声明。
- 不解除 vendored 测试的 feature 门控塞进默认编译面。
- 不让本目录新增 `use crate::tui` / 对 adapter 的非测试引用(现有 adapter 引用仅
  1 处测试)。
- 不手工删除双副本快照的一侧(见契约 6)。

## 深入阅读

- [crate 篇](../../AGENTS.md)(桩层 / fork / feature 面)· [NOTICE](../../../../NOTICE)(上游来源)
- [tests/AGENTS.md](../../../../tests/AGENTS.md)(V 组 / 快照如何被套件消费)
