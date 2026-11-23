# AGENTS.md — tui/(事件循环与枚举通道)

> **何时读我**:改事件循环、按键路由、UiState 状态机、协议事件投影或 slash 枚举通道
> 分发前,先读本篇。本篇契约域 = `tui/` + src 根的 `events.rs` / `adapter.rs` /
> `terminal.rs`(后三者与事件循环内聚,故归本篇;目录位置在 src 根)。

## 模块概述

`tui/` 只有 `mod.rs`(约 6.7K 行)与 `conversion.rs`(约 1K 行):本地事件循环、
布局与 slash 枚举通道分发。依赖方向严格单向:`tui` → `tui_core` / `adapter` /
`events` / `history_render` / `terminal`,**任何模块不得反向引用 `crate::tui`**。

## 核心文件导航

| 文件 | 职责 |
|---|---|
| `tui/mod.rs` | `run_async`(:43)事件循环入口;`handle_event`(:660);`handle_key`(:2691);`draw`(:4341)/ `draw_fullscreen`(:4535);`dispatch_slash_command`(:1756)/ 带参版(:1974);`mod conversion` 声明(:4781) |
| `tui/conversion.rs` | 协议事件 → UI 事件的转换与内容格式化(从 mod.rs 抽出的唯一转换边界) |
| `events.rs` | `UiState`(:152)、`LoopState`(:137)、`PendingApproval`(:20)、`PlanApprovalState`(:33) |
| `adapter.rs` | `UiEventKind`(:56)、`UiEvent`(:241)、`UiHistoryItem`(:246)、状态归约器 `apply_event`(:359,返回需 flush 进 scrollback 的行) |
| `terminal.rs` | `TerminalGuard` / `poll_event` / `is_exit_key`(fork 版 crossterm 封装) |

## 本地契约 / 同步点

1. **枚举通道只实现部分命令**:`dispatch_slash_command` 对未接线命令落兜底文案
   「/xxx: 此命令在当前构建中尚未接入分发」(无参 `:1961` / 带参 `:2147`;
   `/skills` `:1941`、`/hooks` `:1946`、`/side` `:2129-2132` 为硬编码未接入)。
   **该文案是 S 组 placeholder 断言锚点,改字必须同步
   `tests/functional/test_slash.py` 的 MATRIX**。
2. **`UiState` 是共享可变状态**:`handle_event` / `dispatch_slash_command` /
   `handle_key` 三处共享(`events.rs:323` 注释),改字段先核对三个消费面。
   流式字段 `committed_live_len`(:176)/ `segment_start`(:183)是轮次边界契约
   (live 区按 tool 边界切片成 `StreamedAgent`),勿随手重置。
3. **`background_tasks: Mutex<JoinSet>`** 在主循环 spawn / abort,abort 时机与
   轮次中断语义绑定 —— 改动必须跑 W 组(中断 / 容错)用例。
4. **legacy 通道分界**:文本提交路径走 `history_render::dispatch_slash`(`:3413`
   调用点;`/clear` `:5673`、embed 的 `/prompt` `:2139` 另有调用),白名单与结局
   契约单点在 [history_render 篇](../history_render/AGENTS.md);两条通道行为
   不一致,新增 / 改命令两边都要看(根篇关键约束 8)。
5. **embed 模式**另有一条内嵌分发 `tui_core/chatwidget/slash_dispatch.rs`,
   契约归 [tui_core 篇](../tui_core/AGENTS.md)。
6. **v1.12 挂起交互的权威事实在引擎**:`InteractionResolved`(conversion 投影
   → apply_event)按 id 无条件清理五类挂起态(审批 / plan 审批 / plan 进入
   确认 / 问答 / 询问;plan 的 request_id 即 plan_id)—— TTL 过期 / 取消 /
   中断 / 通道关闭这些了结路径**没有**别的回执事件,`UiState` 新增挂起态
   字段时必须同步该清理臂(适配 `conversion.rs` 单测 + W 组行为);
   非 `Answered` 结局补 Notice 说明弹条去向。`Event.seq`(会话扇出单调
   序号)面向断线重连客户端,TUI 内嵌直连无缺口面,有意忽略(conversion
   头注释)。
7. **引擎 additive 事件族的承接口径**(指针 e246299 起):`EventMsg` 新增变体
   属 additive(协议注释「客户端须容忍未知 type」),但 `convert_event` 的
   match **无通配臂** —— 引擎每加变体本仓必须显式补臂,否则 E0004 编译失败
   (有意的接线门:防新事件静默丢失)。当前承接:诊断推送族(`StreamRecovery*`
   4 臂 / `CacheWarming`)→ Notice 进度行;`McpElicitation` → Notice 说明宿主已
   fail-closed cancel(交互面未建,见 [crate 篇](../../AGENTS.md) 契约 11)。
   三者均有 conversion 单测锚定文案关键字。

## 禁止事项

- 不新增 `use crate::tui`(tui_core / adapter / events / history_render 内禁止反向依赖)。
- 协议转换只走 `conversion.rs`,不在 `handle_key` / `draw` 里散写转换逻辑。
- 不绕过 `UiState` 另起全局可变状态;不直接操作 `live` 切片绕开流式字段契约。

## 注意事项

- `terminal.rs` 的 Sixel 透传是**诚实降级**(TODO,返回 `None`),不要假装修好了。
- `picker/` 由本模块 `draw` 叠加调用,选择器契约见 [crate 篇](../../AGENTS.md) §picker。

## 深入阅读

- [tui_core 篇](../tui_core/AGENTS.md) · [crate 篇](../../AGENTS.md) · [tests/COVERAGE.md](../../../../tests/COVERAGE.md) §S / §T / §K 组
