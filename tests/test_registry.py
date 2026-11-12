"""test_registry.py —— ZhuYe-CLI(跨平台 Rust TUI)任务注册表。

任务 ID 与 PLAN.md 第 3 章对应;映射到仓库现有测试入口:
能力覆盖层(tests/functional/,pytest + pty)、Rust 侧(tests/rust/)、
覆盖自审(tools/coverage_audit.py)与 live 层。coverage 语义同 Reflect-Agent。
"""

TASKS: dict[str, dict] = {
    # ---- 公共基座 ----
    "shared.harness": {
        "builtin": "shared.harness",
        "note": "测试基座自检:Python 版本 / 工具链可用性 / 报告目录可写",
    },

    # ---- 静态与纯逻辑(PLAN §3.1,T-U01) ----
    "tui.static": {
        # T-U01:fmt + clippy + 编译(workspace_tests.sh 含 cargo test/clippy/fmt/doc)
        "command": ["tests/rust/workspace_tests.sh"],
        "note": "T-U01:workspace 测试(cargo test + clippy + fmt + doc)",
        "timeout_s": 3600,
    },

    # ---- 输入 / 状态 / 渲染组件(PLAN §3.1,T-U02~T-U17) ----
    "tui.unit.input": {
        # T-U02~T-U08:键盘交互 / 历史 / vim / 补全相关断言
        "command": ["tests/functional/run.sh"],
        "note": "T-U02~T-U08:输入与快捷键(test_keys.py 等承担;入口为能力覆盖层)",
        "timeout_s": 3600,
    },
    "tui.unit.state": {
        # T-U09~T-U11:状态投影 / 会话切换 / 滚动
        "command": ["tests/functional/run.sh"],
        "note": "T-U09~T-U11:状态与滚动(test_session.py / test_turn.py 承担)",
        "timeout_s": 3600,
    },
    "tui.component.render": {
        # T-U12~T-U17:渲染 / markdown / 主题 / resize / 快照
        "command": ["tests/functional/run.sh"],
        "note": "T-U12~T-U17:渲染与主题(test_render.py / test_overlay.py 承担)",
        "timeout_s": 3600,
    },

    # ---- Runtime 接线与真实 PTY(PLAN §3.2,T-E01~T-E20) ----
    "tui.contract.runtime": {
        # CLI 参数矩阵(headless CLI 语义断言)
        "command": ["tests/rust/cli_core.sh"],
        "note": "headless CLI 参数矩阵(T 侧接线前置)",
        "timeout_s": 1800,
    },
    "tui.integration.runtime": {
        # session / traces 参数矩阵与输出契约
        "command": ["tests/rust/cli_modules.sh"],
        "note": "session / traces 参数矩阵与输出契约",
        "timeout_s": 1800,
    },
    "tui.e2e.terminal": {
        # T-E01~T-E19:真实二进制 + pty 的行为级断言(能力覆盖层)
        "command": ["tests/run_all.sh", "functional"],
        "note": "T-E01~T-E19:能力覆盖层(真二进制 + pty);T-E20 ConPTY 见 tui.platform",
        "timeout_s": 5400,
        "os": ["linux", "darwin"],
        "required_on": ["linux", "darwin"],
    },

    # ---- 终端生命周期、安全与兼容(PLAN §3.3,T-P01~T-P15) ----
    "tui.platform": {
        # T-P01~T-P05 信号 / 恢复边界在能力覆盖层 test_flow.py。
        "command": ["tests/functional/run.sh"],
        "note": "T-P01~T-P05:Unix 生命周期与信号(test_flow.py 承担)",
        "timeout_s": 3600,
        "os": ["linux", "darwin"],
    },
    "tui.platform.windows": {
        # T-E20:Windows ConPTY 专项,直接启动 Python 原生适配器。
        "command": ["python", "tests/conpty.py"],
        "note": "T-E20:Windows ConPTY 启动 / 输入 / Ctrl+C 冒烟",
        "timeout_s": 1200,
        "os": ["windows"],
        "required_on": ["windows"],
    },
    "tui.security": {
        # T-P09 ANSI/OSC 注入、T-P10 历史脱敏:能力覆盖层内含核心断言
        "command": ["tests/functional/run.sh"],
        "note": "T-P09/T-P10:注入与脱敏(W15 与历史脱敏断言);扩展矩阵待补",
        "timeout_s": 3600,
    },
    "tui.fault": {
        "command": ["tests/functional/run.sh"],
        "note": "T-P02~T-P05:坏配置 / 坏模型 / SIGTERM / 非 TTY 等故障与恢复",
        "timeout_s": 3600,
        "os": ["linux", "darwin"],
    },
    "tui.coverage.audit": {
        # 覆盖自审:slash 命令 × 测试矩阵差集核对(防占位项假绿)
        "command": ["tests/run_all.sh", "audit"],
        "note": "slash 命令覆盖自审(--strict)",
        "timeout_s": 1800,
    },

    # ---- 性能、长稳与发布(PLAN §3.4,T-Q01~T-Q08) ----
    "tui.performance": {
        "command": ["python", "tests/performance.py"],
        "note": "T-Q01~T-Q03:启动 / 长历史 / 输出洪峰性能基线",
        "timeout_s": 1800,
        "os": ["linux", "darwin"],
        "required_on": ["linux", "darwin"],
    },
    "tui.soak": {
        "command": ["python", "tests/soak.py"],
        "note": "T-Q04:持续 resize / redraw 的存活与 RSS 趋势",
        "timeout_s": 3600,
        "os": ["linux", "darwin"],
        "required_on": ["linux", "darwin"],
    },
    "tui.release": {
        "command": ["python", "tests/release.py"],
        "note": "T-Q05~T-Q07:发布包结构 / 版本 / 本机执行与安装升级卸载冒烟",
        "uses_artifact": True,
        "timeout_s": 600,
    },

    # ---- live 层(真实 LLM,默认 SKIP) ----
    "tui.live": {
        "coverage": "conditional",
        "command": ["tests/run_all.sh", "live"],
        "note": "live 端到端(需真实 LLM + API key)",
        "enabled": False,
    },
}

SUITES: dict[str, list[str]] = {
    # PR:静态、逻辑、快照、核心 PTY、覆盖审计
    "pr": [
        "tui.static", "tui.coverage.audit", "tui.e2e.terminal",
    ],
    # 冒烟:T-E01/T-E02/T-E04/T-E05/T-E07/T-E08 对应能力覆盖层核心路径
    "smoke": ["tui.e2e.terminal"],
    # 主分支:PR 层 + CLI 参数面 + 各组件定向
    "main": [
        "tui.static", "tui.coverage.audit", "tui.e2e.terminal",
        "tui.contract.runtime", "tui.integration.runtime",
        "tui.unit.input", "tui.unit.state", "tui.component.render",
        "tui.platform", "tui.platform.windows", "tui.security",
    ],
    # 夜间:终端组合、洪峰、生命周期、长稳
    "nightly": ["tui.fault", "tui.performance", "tui.soak", "tui.live"],
    # 发布候选:最终二进制与安装(未建立渠道前为 pending,报告暴露缺口)
    "release": ["tui.static", "tui.e2e.terminal", "tui.release"],
}

CASE_INDEX: dict[str, str] = {
    **{f"B{i:02d}": "shared.harness" for i in range(1, 17)},
    # T-U01 静态
    "T-U01": "tui.static",
    # T-U02~T-U08 输入
    **{f"T-U{i:02d}": "tui.unit.input" for i in range(2, 9)},
    # T-U09~T-U11 状态
    **{f"T-U{i:02d}": "tui.unit.state" for i in range(9, 12)},
    # T-U12~T-U17 渲染
    **{f"T-U{i:02d}": "tui.component.render" for i in range(12, 18)},
    # T-E01~T-E19 PTY / ConPTY
    **{f"T-E{i:02d}": "tui.e2e.terminal" for i in range(1, 20)},
    "T-E20": "tui.platform.windows",
    # T-P 平台 / 安全
    **{f"T-P{i:02d}": ("tui.security" if i in (9, 10) else "tui.platform")
       for i in range(1, 16)},
    # T-Q 性能与发布
    **{f"T-Q{i:02d}": ("tui.performance" if i <= 4 else "tui.release")
       for i in range(1, 9)},
}
