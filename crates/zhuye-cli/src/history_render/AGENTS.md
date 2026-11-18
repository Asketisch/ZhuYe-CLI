# AGENTS.md — history_render(历史行渲染与 legacy 通道)

> **何时读我**:改 slash **legacy 通道**(白名单 / `dispatch_slash`)、statusline、
> 历史行渲染(cells / marker / notify)之前,先读本篇。本篇契约域 =
> `history_render.rs`(src 根文件)+ `history_render/` 目录的**双形态模块**。

## 模块概述

新旧两套渲染的中介层:向上被 `adapter` / `events` / `tui/mod.rs` / `tasks_pager`
消费,向下复用 `tui_core::history_cell`;同时承载 slash 命令的 **legacy 文本通道**
(字符串匹配,与枚举通道并存且行为不一致,根篇关键约束 8)。

## 核心文件导航

| 文件 | 职责 |
|---|---|
| `history_render.rs`(src 根,约 1.7K 行) | legacy 通道本体:`SlashOutcome`(:221)、白名单 `is_legacy_passthrough_slash`(:445)、`legacy_popup_commands`(:470)、`dispatch_slash`(:490) |
| `history_render/statusline.rs`(约 1K 行) | 状态栏渲染(theme_picker 回归测试锚点 :427) |
| `history_render/cells/` `marker/` `notify/` | 历史行单元 / 标记 / 通知渲染 |

## 本地契约 / 同步点

1. **白名单 12 名与 match 臂必须同步**(`history_render.rs:434-444` 头注释):
   `is_legacy_passthrough_slash` 只收录 `SlashCommand` 枚举里**没有**的命令名
   (`help` / `?` / `tasks` / `agents` / `cost` / `exit-plan` / `exitplan` / `mode`
   / `effort` / `ref` / `rm-session` / `split`),两套都有的命令走枚举路径不进白名单。
   新增 legacy 命令 = 白名单 + `dispatch_slash` match 臂一起加。
2. **composer 放行回环**:白名单的消费点在
   `tui_core/bottom_pane/chat_composer/slash_input.rs:88` 与 `:260`(validate_submission
   放行旧命令为合法提交);`legacy_popup_commands()`(8 条,含枚举没有的 `rewind`)
   被 `tui_core/bottom_pane/command_popup.rs:105` 消费进命令弹窗。改这三处任一,
   都要回到本模块核对两个集合。
3. **本模块是 `coverage_audit.py` 的解析输入**:`is_legacy_passthrough_slash` 白名单、
   `dispatch_slash` 的 `/` 开头字面量 match 臂、`legacy_popup_commands` 表都会被
   机械提取去做差集(机制单点见 [tests/AGENTS.md](../../../../tests/AGENTS.md));
   提取按**行缩进边界**切函数体,改代码须保持 rustfmt 风格,改完跑
   `make coverage-audit`。
4. `SlashOutcome` 五类结局(Handled / Clear / ConfirmClear / Exit / SubmitAsUser …)
   是 S 组断言的分类口径,新增结局类别同步 MATRIX 与 COVERAGE.md §S 组。

## 禁止事项

- 不把枚通道已有的命令收进白名单(会让该命令绕过枚举分发)。
- 不改 `dispatch_slash` 的 match 臂字面量而不跑 `make coverage-audit`。
- 不在本模块引入对 `crate::tui` 的依赖(保持中介层向下单向)。

## 注意事项

- `history_render.rs` 与 `history_render/` 是同一模块的文件 + 目录双形态
  (声明见 `history_render.rs:7-10`),新增子模块落目录、通道逻辑留根文件。

## 深入阅读

- [tui 篇](../tui/AGENTS.md)(枚举通道与调用点)· [tests/AGENTS.md](../../../../tests/AGENTS.md)(差集门禁)
