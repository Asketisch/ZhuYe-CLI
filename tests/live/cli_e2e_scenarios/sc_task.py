"""sc_task.py —— 真实任务全链路场景(消费配置 toml 的 [task] 段)。

与 ZhuYe 主仓 cli_e2e.toml 的任务配对:prompt 在 plan 模式下作为首个
任务下发,批准 plan 进入开发回合,continue_prompt 作为收尾指令下发;
验收期望 expect_artifact / expect_commit_subject 与任务绑死,换任务
必须同步改 toml 的 [task] 段。**本质依赖真实模型**(needs_model):
plan 的提出 / 批准 / 执行由模型行为驱动,mock 脚本无法确定性构造。
"""

from __future__ import annotations

import cli_e2e_lib as lib

# 工作区夹具:任务 prompt 要求「阅读本工作区的 README.md」后再建介绍页,
# 夹具给合成内容(产品名 / 定位 / 特性 / 快速开始),不喂真实仓库内容
# 上游可能记录 prompt。与 ZhuYe 主仓 cli_e2e.toml [task].prompt 配对。
README_FIXTURE = """# 烽火 Fanfire

烽火(Fanfire)是 ZhuYe 家族的终端火种:一条命令点亮终端 AI 工作流。

- **即点即燃**:零配置启动,开箱即用的 mock 与免费模型车道
- **烽火台接力**:会话落盘 / 续作回放,长任务随时断点续传
- **火情瞭望**:traces 与遥测内置,每一次模型调用可回溯

快速开始:

```bash
zhuye-cli
```
"""


def _on_plan_auto(driver: lib.TuiDriver, screen: str) -> bool:
    """计划批准 overlay → 输入 1(Auto:批准并进入开发,可写盘)。

    场景自带处理器(先于兜底应答消费)—— 批准与否是本场景的核心语义,
    不走「工具审批兜底放行」那条路。
    """
    if lib.PLAN_ANCHOR in screen:
        driver.approvals_seen.append("plan")
        driver.s.type_text("1")
        return True
    return False


def run_task_main(cx: lib.Ctx) -> None:
    """任务全链路:plan 模式出 plan → 批准执行 → 产物 → git 提交 → 落盘。"""
    scalars = cx.cfg["_scalars"]
    prompt = cx.p("prompt") or (cx.cfg.get("task") or {}).get("prompt") or ""
    continue_prompt = cx.p("continue_prompt") or (cx.cfg.get("task") or {}).get("continue_prompt") or ""
    expect_artifact = cx.p("expect_artifact") or scalars.get("EXPECT_ARTIFACT") or ""
    expect_commit = cx.p("expect_commit") or scalars.get("EXPECT_COMMIT") or ""
    if not prompt:
        cx.checks.add("任务 prompt 存在", False, "配置 toml 缺 [task].prompt")
        return

    def setup(ws: str) -> None:
        cx.write_file(ws, "README.md", README_FIXTURE)

    env = cx.env(setup=setup)
    with cx.tui(env) as driver:
        cx.checks.add("TUI 启动到稳定首屏", True)
        cx.set_effort(driver, cx.effort())
        # 进 plan 模式:HUD 出现 [plan] 标记(composer 占位符同步变化)。
        driver.send("/mode plan")
        driver.wait_screen("[plan]", timeout=30.0)
        cx.checks.add("进入 plan 模式([plan] 标记)", True)

        # 下发任务 → 等计划批准 overlay(plan_ready 打开的面板)。
        driver.send(prompt)
        try:
            driver.wait_screen(lib.PLAN_ANCHOR, timeout=cx.timeout())
            cx.checks.add("计划批准 overlay 出现", True)
        except lib.WaitTimeout as e:
            cx.checks.add("计划批准 overlay 出现", False, str(e))
            return
        # [1] Auto = 批准并自动进入开发(AcceptEdits,可写盘)。
        driver.s.type_text("1")

        # 开发阶段:等期望产物出现;期间兜底放行工具审批、监视限流。
        if expect_artifact:
            try:
                produced = lib.wait_for_file(env, expect_artifact, cx.timeout(), driver=driver)
                cx.checks.add(f"产物落盘({expect_artifact})", produced,
                              f"工作区 {env.ws} 中未出现")
            except lib.RateLimited:
                raise
        else:
            produced = True

        # 收尾指令:等开发回合收尾后再下发(与 wire 侧 submission_closed 语义对齐)。
        if produced:
            try:
                driver.wait_idle(cx.timeout())
                if continue_prompt:
                    driver.turn(continue_prompt, timeout=cx.timeout())
                cx.checks.add("收尾回合完成", True)
            except lib.WaitTimeout as e:
                cx.checks.add("收尾回合完成", False, str(e))

        # git 验收:sandbox 内 init / add / commit 全部成功(快照归属回归点)。
        if expect_commit:
            subjects = cx.commit_subjects(env.ws)
            cx.checks.add(f"提交主题匹配({expect_commit})", expect_commit in subjects,
                          f"实际提交:{subjects or '(无提交 —— git init/commit 未完成)'}")

        # 会话落盘验收:开发回合的 assistant 消息与 token 用量必须已记录。
        stats = env.stats()
        cx.checks.add("assistant 消息落盘 ≥1", stats["assistant_messages"] >= 1,
                      f"实际 {stats['assistant_messages']} 条")
        cx.checks.add("token 用量落盘(total > 0)", stats["usage"]["total_tokens"] > 0,
                      f"实际 {stats['usage']}")
        cx.checks.add("无 panic 痕迹", not driver.s.has_panic(),
                      driver.s.raw_output()[-800:] if driver.s.has_panic() else "")
        cx.metrics["usage"] = stats["usage"]
        cx.metrics["commits"] = cx.commit_subjects(env.ws)


TASK_MAIN = lib.Scenario(
    "task_main",
    "真实任务全链路(plan → 批准 → 产物 → 提交)",
    run_task_main,
    needs_model=True,
    defaults={
        "prompt": "",
        "continue_prompt": "",
        "expect_artifact": "",
        "expect_commit": "",
    },
)
