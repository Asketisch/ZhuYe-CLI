"""sc_turn.py —— 单轮能力场景:问答回合 / 工具调用与审批。

两个场景都提供 mock 脚本(自检模式确定性跑通),真模型模式下同一套
结构化断言(屏幕结构标记 / 落盘文件 / 终态语义),不断言模型自由文本。
"""

from __future__ import annotations

import json

import cli_e2e_lib as lib

# mock 自检的确定性回执标记(屏幕断言用;真模型模式不断言文本)。
BASIC_MARKER = "E2E-OK-4021"
TOOL_MARKER = "zhuye-e2e-tool-probe"


def _mock_script_basic(_params: dict) -> list[str]:
    """单轮问答的脚本化回复:一次模型调用,一行纯文本。"""
    return [json.dumps({"type": "text", "text": BASIC_MARKER}, ensure_ascii=False)]


def _mock_script_tool(_params: dict) -> list[str]:
    """工具调用的脚本化回复:先调 bash,收尾一条纯文本(每次调用一行)。"""
    return [
        json.dumps(
            {"type": "tool_call", "name": "bash", "args": {"command": f"echo {TOOL_MARKER}"}},
            ensure_ascii=False,
        ),
        json.dumps({"type": "text", "text": "完成"}, ensure_ascii=False),
    ]


def run_turn_basic(cx: lib.Ctx) -> None:
    """单轮问答:回合收尾 / assistant 消息落盘 / token 用量记录 / 无 panic。"""
    env = cx.env()
    if cx.mock:
        prompt = cx.p("prompt", "请只回复这串标记:" + BASIC_MARKER)
    else:
        prompt = cx.p("prompt", "1+1 等于几?只回答数字,不要调用任何工具。")
    with cx.tui(env) as driver:
        cx.checks.add("TUI 启动到稳定首屏", True)
        driver.turn(prompt, timeout=cx.timeout())
        cx.checks.add("回合收尾(spinner 消失)", True)
        cx.checks.add("无 panic 痕迹", not driver.s.has_panic(),
                      driver.s.raw_output()[-800:] if driver.s.has_panic() else "")
        stats = env.stats()
        cx.checks.add("assistant 消息落盘 ≥1", stats["assistant_messages"] >= 1,
                      f"实际 {stats['assistant_messages']} 条")
        cx.checks.add("token 用量落盘(total > 0)", stats["usage"]["total_tokens"] > 0,
                      f"实际 {stats['usage']}")
        if cx.mock:
            cx.checks.add("屏幕出现回执标记(mock 确定性)", BASIC_MARKER in screen(driver),
                          "屏幕未出现 " + BASIC_MARKER)
        cx.metrics["usage"] = stats["usage"]


def screen(driver: lib.TuiDriver) -> str:
    """当前屏幕归一化文本(读屏尽力而为)。"""
    return lib.screen_text(driver.s)


def run_turn_tool(cx: lib.Ctx) -> None:
    """工具调用与审批放行:bash echo 的输出上屏,回合收尾落盘。"""
    env = cx.env()
    prompt = cx.p("prompt", "请用 bash 工具执行 echo " + TOOL_MARKER + ",把命令输出原样告诉我。")
    with cx.tui(env) as driver:
        cx.checks.add("TUI 启动到稳定首屏", True)
        driver.turn(prompt, timeout=cx.timeout())
        cx.checks.add("回合收尾(spinner 消失)", True)
        cx.checks.add("无 panic 痕迹", not driver.s.has_panic(),
                      driver.s.raw_output()[-800:] if driver.s.has_panic() else "")
        stats = env.stats()
        cx.checks.add("assistant 消息落盘 ≥1", stats["assistant_messages"] >= 1,
                      f"实际 {stats['assistant_messages']} 条")
        if cx.mock:
            # mock 确定性:bash 一定被调用,回执一定回显。
            cx.checks.add("工具输出上屏(mock 确定性)", TOOL_MARKER in screen(driver),
                          "屏幕未出现 " + TOOL_MARKER)
            cx.checks.add("工具成功行上屏(✓ bash)", "✓ bash" in screen(driver),
                          "屏幕未出现 ✓ bash 成功行")
        else:
            # 真模型:没按提示词调工具是「模型没配合」,判跳过 —— TUI 的
            # 审批与渲染链路本身由 mock 模式确定性覆盖。
            ran = TOOL_MARKER in screen(driver) or "✓ bash" in screen(driver)
            if ran:
                cx.checks.add("工具调用与输出上屏", True)
            else:
                lib.skip_all(cx, ["工具调用与输出上屏"], "真模型未按提示词调用 bash 工具")
        cx.metrics["usage"] = stats["usage"]


TURN_BASIC = lib.Scenario(
    "turn_basic",
    "单轮问答:回合收尾与落盘",
    run_turn_basic,
    defaults={"prompt": ""},
    mock_script=_mock_script_basic,
)

TURN_TOOL = lib.Scenario(
    "turn_tool",
    "工具调用与审批放行",
    run_turn_tool,
    defaults={"prompt": ""},
    mock_script=_mock_script_tool,
)
