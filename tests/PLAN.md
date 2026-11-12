# 测试实施计划

本文件是 TUI 统一调度器的权威任务目录。TUI 是独立产品,只验证输入、渲染、
终端生命周期和对 Runtime 的消费方式;Agent 内部能力仍由上游能力层承担。

## 1. 公共基座

非 live 套件使用 mock 模型、隔离 HOME/cwd、真实 release 二进制、PTY 屏幕
锚点和进程组超时。缺 Python、pytest 或 pyte 属于基础设施错误;Windows 当前
ConPTY 专项使用 pywinpty + pyte 驱动同一份 release 二进制;Windows 缺依赖、
缺二进制或用例失败都阻断。

结果状态固定为 `pass`、`fail`、`skip`、`infra_error`、`timeout`、`pending`。
`pending` 与基础设施错误返回 2;断言失败或超时返回 1;dry-run 全部为 `skip`。

## 2. TUI 任务

| 任务 | 覆盖目标 | 状态 |
|---|---|---|
| `tui.static` | workspace 测试、Clippy、格式、文档 | 映射现有 Rust 套件 |
| `tui.unit.input` / `tui.unit.state` | 输入、按键、状态、滚动 | 映射能力层 |
| `tui.component.render` | Markdown、diff、主题、宽字符 | 映射渲染能力层 |
| `tui.contract.runtime` / `tui.integration.runtime` | CLI 接线、session/traces | 映射参数矩阵 |
| `tui.e2e.terminal` | Unix PTY 真二进制核心交互 | Linux/macOS 必测 |
| `tui.platform` / `tui.platform.windows` | 终端生命周期、信号、ConPTY | Unix PTY 与 Windows ConPTY 均映射 |
| `tui.security` | ANSI/OSC 注入与历史脱敏 | 核心映射,专项待补 |
| `tui.coverage.audit` | slash 命令覆盖差集 | 映射 strict 审计 |
| `tui.fault` | 坏配置 / 坏模型 / 信号 / 非 TTY 故障 | 能力覆盖层映射 |
| `tui.performance` / `tui.soak` | 性能与长稳 | release 二进制 + PTY 基线映射 |
| `tui.release` | 最终二进制、包结构、版本、执行与安装升级卸载 | 发布产物审查映射 |
| `tui.live` | 真实 LLM 端到端 | 条件项 |

`CASE_INDEX` 保留 T 系列编号。渲染缓冲区测试不能替代真实 PTY;真实终端、
IME 和复用器矩阵在专项落地前不得计为通过。

## 3. 执行与报告

选集固定为 `pr`、`smoke`、`main`、`nightly`、`release`。PR 必须包含静态、
覆盖审计和 Unix PTY 核心层;发布选集中的发布矩阵保持未覆盖并阻断通过。
报告写入 `.test-runner/`,记录被测 HEAD、工作树状态、平台、复现命令、日志和 seed。
