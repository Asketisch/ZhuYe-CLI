"""cli_e2e 场景注册表(TUI 端到端演练)。

每个场景模块导出若干 `lib.Scenario`,这里汇总成 `SCENARIOS`。新增场景:
写模块 → 在此登记 → 视性质补 mock 脚本(needs_model=False 必须提供,
否则自检跳过)。cli_e2e.sh 的场景清单即来自本注册表。

场景分组:
- turn    —— 单轮能力(问答收尾落盘 / 工具调用与审批放行);
- task    —— 真实任务全链路(plan 出 plan / 批准执行 / 产物 / git 提交,
             消费模型配置 toml 的 [task] 段);
- session —— 会话持久化(--resume 回放重建 scrollback / 续作再落盘)。

后续可扩(对齐 ZhuYe 主仓同名套件的分组):compact(/compact 手动压缩)、
goal(/goal 目标模式进 / 出)—— 见 tests/live/README.md 的「扩展点」。
"""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import cli_e2e_lib as lib
from . import sc_session, sc_task, sc_turn

_ALL = (
    # turn
    sc_turn.TURN_BASIC,
    sc_turn.TURN_TOOL,
    # task
    sc_task.TASK_MAIN,
    # session
    sc_session.SESSION_RESUME,
)

SCENARIOS: dict[str, lib.Scenario] = {s.name: s for s in _ALL}
