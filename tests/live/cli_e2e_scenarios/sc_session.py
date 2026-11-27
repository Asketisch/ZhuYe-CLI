"""sc_session.py —— 会话持久化场景:落盘回放与 `--resume` 续作。

mock 与真模型都适用:续作回放的断言落在「第一轮的标记重新出现在屏幕上」
—— 回放重建 scrollback 时,既往 user / assistant 消息原样回填,标记必然
可见,不依赖模型记忆文本(模型自由发挥的断言不可靠)。
"""

from __future__ import annotations

import json
from pathlib import Path

import cli_e2e_lib as lib

# 落盘 / 回放的确定性回执标记(mock 自检;真模型模式下标记在 user 消息里,
# 回放后同样可见)。
RESUME_MARKER = "E2E-RESUME-7788"
CONT_MARKER = "E2E-RESUME-CONT"


def _mock_script_run1(_params: dict) -> list[str]:
    return [json.dumps({"type": "text", "text": "已记住 " + RESUME_MARKER}, ensure_ascii=False)]


def _mock_script_run2(_params: dict) -> list[str]:
    return [json.dumps({"type": "text", "text": "续作回执 " + CONT_MARKER}, ensure_ascii=False)]


def run_session_resume(cx: lib.Ctx) -> None:
    """第一轮落盘 → 退出 → `--resume <tid>` 重启 → 回放可见 → 续作再落盘。"""
    # 首轮用场景注册的脚本;续作段换一份脚本(见下方 env2)。非 mock 模式
    # 不注入任何脚本 —— Ctx.env 只在 mock 自检下消费它们。
    env = cx.env(mock_script=_mock_script_run1(cx.params))
    first_prompt = cx.p(
        "prompt",
        "请只回复这串标记:" + RESUME_MARKER,
    )
    with cx.tui(env) as driver:
        cx.checks.add("首轮 TUI 启动到稳定首屏", True)
        driver.turn(first_prompt, timeout=cx.timeout())
        cx.checks.add("首轮回合收尾", True)
        if cx.mock:
            cx.checks.add("首轮回执上屏(mock 确定性)", RESUME_MARKER in lib.screen_text(driver.s),
                          "屏幕未出现 " + RESUME_MARKER)
        files = env.session_files()
        cx.checks.add("首轮会话 JSONL 落盘", len(files) >= 1, "sessions/ 下没有 jsonl")
        if not files:
            return
        # 会话文件名即 thread id(<tid>.jsonl);--resume 以它为续作目标。
        thread_id = Path(files[-1]).stem
        cx.metrics["thread_id"] = thread_id

    # ── 第二段:--resume 重启(换一份续作脚本;真模型模式该参数不消费)───
    env2 = cx.env(mock_script=_mock_script_run2(cx.params))
    with cx.tui(env2, args=["--resume", thread_id]) as driver:
        cx.checks.add("续作 TUI 启动到稳定首屏", True)
        # 回放重建:首轮的 user / assistant 消息回填 scrollback,标记可见。
        try:
            driver.wait_screen(RESUME_MARKER, timeout=30.0)
            cx.checks.add("续作回放重建历史(首轮标记可见)", True)
        except lib.WaitTimeout as e:
            cx.checks.add("续作回放重建历史(首轮标记可见)", False, str(e))
        driver.turn(
            cx.p("continue_prompt", "继续对话,只回复:续作回执 " + CONT_MARKER),
            timeout=cx.timeout(),
        )
        cx.checks.add("续作回合收尾", True)
        stats = env2.stats()
        cx.checks.add("续作 assistant 消息累计落盘 ≥2", stats["assistant_messages"] >= 2,
                      f"实际 {stats['assistant_messages']} 条")
        if cx.mock:
            cx.checks.add("续作回执上屏(mock 确定性)", CONT_MARKER in lib.screen_text(driver.s),
                          "屏幕未出现 " + CONT_MARKER)
        cx.metrics["assistant_messages"] = stats["assistant_messages"]


SESSION_RESUME = lib.Scenario(
    "session_resume",
    "会话落盘与 --resume 续作回放",
    run_session_resume,
    defaults={"prompt": "", "continue_prompt": ""},
    mock_script=_mock_script_run1,
)
