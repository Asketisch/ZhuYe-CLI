# AGENTS.md — tests/(测试分层与覆盖门禁)

> **何时读我**:新增 / 修改任何测试、动 `COVERAGE.md` 结构、被 `make coverage-audit`
> 卡住,或想确认「这个改动该补什么测试」之前,先读本篇。

## 目录职责

多语言测试体系:能力覆盖层(真二进制 + pty 行为断言)、Rust 参数面、覆盖自审、
live 端到端四类。分层判据与用例规模的正式说明见 [tests/README.md](README.md),
能力 × 覆盖层映射见 [tests/COVERAGE.md](COVERAGE.md)。

## 四个入口(`tests/run_all.sh`)

| 入口 | 语义 | 落点 |
|---|---|---|
| `functional`(默认之一) | 能力覆盖层 + Rust 参数面:真二进制 + pty 行为断言 | `functional/run.sh` → `rust/run.sh` |
| `rust`(默认之一) | cargo test / clippy / fmt / doc 四件套 + CLI 参数矩阵 | `rust/run.sh` |
| `audit`(默认之一) | 覆盖自审:slash 命令 × 矩阵差集 | `tools/coverage_audit.py --strict` |
| `live` | 真实 LLM 端到端(需 API key,手动跑,默认 SKIP) | `live/test_workflow_live.py` |

无参 = 前三入口;入口先 `ensure_release_binary` 保证二进制与源码对齐;全程
`ZHUYE_MODEL=mock/mock-1` + 隔离 HOME(mktemp),不依赖外网与 API key。

## 组字母全表(`functional/`)

| 组 | 文件 | 面 |
|---|---|---|
| B | `test_boot.py` | 启动 / 渲染锚点 / 退出生命周期 / 全屏 |
| S | `test_slash.py` | slash 全量矩阵(MATRIX 单一事实源) |
| O | `test_overlay.py` | 弹层 / 浮层 / 选择器 + 反向门禁 |
| T | `test_turn.py` | 对话轮次 / 流式 / 工具调用 / 回合边界 |
| P | `test_session.py` | 会话持久化 / 恢复 / fork / plan 落盘 |
| C | `test_cli.py` | headless CLI 语义(subprocess + json) |
| K | `test_keys.py` | 键盘交互(权限循环 / 历史召回 / vim) |
| V | `test_render.py` | 渲染与视觉(宽字符 / 折行 / markdown / resize) |
| W | `test_flow.py` | 审批 / AskUser / loop / 恢复边界 / 信号 |
| G | `test_plugin.py` | 插件面(slash 注入 / plugin overlay / 门控插件 disabled_plugins) |
| H | `test_http_mock.py` | 真实 HTTP 链路(进程内 mock Anthropic SSE) |
| U | `test_pty_helpers.py` | 测试设施自检 |
| D | `test_differential.py` | TUI 版本差分(需 `ZHUYE_DIFF_BASELINE/CANDIDATE`,经 `run_differential.sh` 显式启用) |
| L / F | `live/test_workflow_live.py`、`live/test_full_feature_live.py` | live 层(不在默认套件) |

公共设施:`tui_pty.py`(pty 会话)、`conftest.py`、`mock_anthropic_server.py`、
`assert_differential_report.py`。根级还有统一调度器 `runner.py`(三仓接口,
`list` / `run --suite pr/smoke`,注册表 `test_registry.py`)与专项
`performance.py` / `soak.py` / `release.py` / `conpty.py` / `verification/`。

## 覆盖自审机制(`tools/coverage_audit.py`,全仓唯一落点)

- **四输入**:① `src/tui_core/slash_command.rs` 的 `SlashCommand` 枚举(含 strum
  主名与别名);② `src/history_render.rs` 的白名单 + `dispatch_slash` 的 `/` 开头
  match 臂字面量 + `legacy_popup_commands()`;③ `src/picker/*.rs` 的标题字面量;
  ④ `test_slash.py` 的 MATRIX(常量定义 `coverage_audit.py:41-44`)。
- declared = 枚举 ∪ 白名单 ∪ 实现臂;`--strict` 下 declared − tested 非空即
  exit 1;解析失败 exit 2。
- **按行缩进边界提取 Rust 函数体**(非花括号配平)—— 被解析的源码必须保持
  rustfmt 风格,奇怪缩进会让提取漏臂。
- `probe_slash_commands.py` 是探针(产出事实清单供 COVERAGE.md 实证),不做断言。
- `functional/test_coverage_audit.py` 是门禁**自身**的行为级用例(无未覆盖差集 /
  MATRIX 覆盖全 declared / picker 全解析 / 枚举计数稳定),随 functional 入口跑,
  不单设组字母。

## 本地契约 / 同步点

1. **行为级断言只写在 `functional/`**:`tests/rust/*.sh` 只管**命令行面**
   (`cli_core.sh` 顶层参数 / `cli_modules.sh` session+traces 参数 /
   `build_tooling.sh` 构建工具面),不重复写行为断言。
2. 新增 slash 命令必须补 `test_slash.py` 的 MATRIX(结局类别 + 期望屏幕文本),
   否则 `make coverage-audit` 失败 —— 双通道细节见 history_render / tui 篇。
3. `COVERAGE.md` 随能力增删同步:新增能力补「能力 × 覆盖层」行,移除入
   「未覆盖项」,缺陷记「已知缺陷」;目录结构变更同步 `README.md` 的目录表。
4. CI 门禁:`.github/workflows/ci.yml` 的 `functional-suite` job 跑
   `./tests/run_all.sh functional`;`nightly-quality` 含 TUI 差分 —— 改套件结构
   先看这两个 job 的调用面。
5. live 用例必须留在 `live/` 且默认 SKIP,不得混进离线套件。

## 禁止事项

- 不在测试里依赖外网 / 真实 API key / 真实 `~/.zhuye`。
- 不用「退出码 0 / 不崩」充当能力断言(根篇检查表 1)。
- 不改 `run_all.sh` 四入口语义而不更新根 AGENTS.md 常用命令与 README。

## 深入阅读

- [tests/README.md](README.md)(分层判据)· [tests/COVERAGE.md](COVERAGE.md)(矩阵)· [tests/PLAN.md](PLAN.md)(实施计划与调度器接口)
