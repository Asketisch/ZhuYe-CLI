# AGENTS.md — cli/(headless 子命令)

> 何时读我:改 headless 子命令(session / traces / discussion / security /
> workspace)或 `src/bin/main.rs` 的 clap 路由之前,先读本篇。

## 模块概述

headless 子命令实现:执行完即退出,不进 TUI。**只走 submodule 公开 API**
(zhuye-rollout / zhuye-telemetry / zhuye-protocol / zhuye-fs / zhuye-discussion /
zhuye-integration),不依赖 `tui_core`(渲染层与 headless 无关)。
`src/bin/main.rs` 是 thin binary:clap 路由,无子命令 → TUI,子命令 → 本模块。

## 核心文件导航

| 文件 | 职责 |
|---|---|
| `cli/mod.rs` | 子命令模块声明与公共约定(参照 headless 侧实现) |
| `cli/session.rs`(约 1.1K 行) | `session` 子命令:ls / show / rm / fork / rename / export / stats(消费 zhuye-rollout 索引与统计) |
| `cli/traces.rs` | `traces` 子命令:ls / show / spans(zhuye-telemetry 读取) |
| `cli/discussion.rs` | `discussion run`:转发 `zhuye_discussion::cli::run` |
| `cli/security.rs` | `security audit`:转发 `zhuye_integration::security_scan` |
| `cli/workspace.rs` | `workspace clone`:git clone 辅助 |
| `src/bin/main.rs` | clap 路由与各入口的 runtime 选择 |

## 本地契约 / 同步点

1. **聚合逻辑复用引擎侧,不在本仓库复刻**:会话统计走 `zhuye_rollout` 的共享
   API(session_stats 等),traces 走 `zhuye_telemetry` 读取口 —— 两边各持一份
   聚合会漂移。
2. **落盘统一 `zhuye_fs::write_atomic`**(如 export 导出,`cli/session.rs:414`),
   不手写 temp+rename。
3. **输出契约被双层断言**:语义断言在 `tests/functional/test_cli.py`(C 组,
   subprocess + json 解析),参数面在 `tests/rust/cli_modules.sh` /
   `cli_core.sh` —— 新增 / 改子命令的参数与输出,两处都要同步。
4. **runtime 选择**(细节单点见 [crate 篇](../../AGENTS.md) 契约 5):TUI 路径
   `current_thread`;`discussion` 例外必须多线程 `Runtime::new()`
   (`src/bin/main.rs:224-229`,跨 task 等待,current_thread 死锁)。新增 headless
   入口按是否跨 task 等待对号入座。
5. 新增子命令同步面:README 的 CLI 子命令章节 + C 组用例 + 参数面脚本 +
   `tests/COVERAGE.md` §3(CLI 子命令清单)。

## 禁止事项

- 不引入对 `tui_core` / `crate::tui` 的依赖(headless 产物不拉渲染层)。
- 不复刻 submodule 已提供的聚合 / 读取逻辑。
- 不新增子命令而不同步第 5 条的四个落点。

## 深入阅读

- [crate 篇](../../AGENTS.md)(runtime 铁律 / write_atomic)· [tests/AGENTS.md](../../../../tests/AGENTS.md)(C 组与参数面分工)
