"""TUI 版本差分:baseline/candidate 两个真实二进制上的确定性场景对比。

定位(工作区 VERIFICATION_ENHANCEMENT_PLAN.md P2):

- 同一 mock 脚本 + 隔离沙箱,分别驱动两个版本的 TUI 真二进制(真实 PTY),
  比较 rollout 记录类型、工作区副作用、屏幕行为标记与 panic 痕迹;
- **屏幕比较用「标记断言」而非全文比对**:版本号、token 计数等发布期
  合法差异不制造噪声;回复上屏、工具副作用、恢复回填、panic 等行为
  语义漂移必报 —— 这是「语义差分」在 TUI 场景的落点;
- 纯逻辑自测(场景比较器)始终运行,不依赖二进制;真实差分需要显式提供
  `ZHUYE_DIFF_BASELINE` / `ZHUYE_DIFF_CANDIDATE`,由条件任务
  `tui.contract.differential`(ZHUYE_DIFF=1)启用。
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from tui_pty import (
    Sandbox,
    TuiSession,
    text_reply,
    tool_call,
    write_mock_script,
)

ROOT = Path(__file__).resolve().parents[2]

# 诊断投影输出目录(差分运行时写入;纯自测不写盘)。
DEFAULT_OUTPUT = ROOT / "target" / "verification-tui"

# 差分场景:与核心仓 tests/verification 的场景一一对应(同一事故面)。
SCENARIOS: list[dict] = [
    {
        "id": "turn_basic",
        "script_lines": [text_reply("差分验证回复")],
        "prompt": "差分提问",
        "wait_marker": "差分验证回复",
        "expect_screen": ["差分验证回复", "差分提问"],
        "forbid_screen": ["panicked at"],
        "resume": False,
    },
    {
        "id": "tool_write_side_effect",
        "script_lines": [
            tool_call("write", {"path": "diff-out.txt", "content": "差分数据"}),
            text_reply("写入完成"),
        ],
        "prompt": "写文件",
        "wait_marker": "写入完成",
        "expect_screen": ["写入完成"],
        "forbid_screen": ["panicked at"],
        "resume": False,
    },
    {
        "id": "resume_history",
        "script_lines": [text_reply("第一轮检查点")],
        "prompt": "记住第一轮",
        "wait_marker": "第一轮检查点",
        "expect_screen": ["第一轮检查点", "记住第一轮", "已恢复历史会话"],
        "forbid_screen": ["panicked at"],
        "resume": True,
    },
]


# ── 纯函数层:场景投影与比较 ─────────────────────────────────────────────


def parse_rollout_types(session_files: list[Path]) -> list[str]:
    """会话 JSONL 记录类型序列(跨版本持久化兼容的语义投影)。"""
    types: list[str] = []
    for path in session_files:
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.strip():
                types.append(json.loads(line).get("type", ""))
    return types


def workspace_snapshot(cwd: Path) -> dict[str, str]:
    """工作区最终快照(相对路径 → 文本),排除测试设施自身产物。"""
    files: dict[str, str] = {}
    for path in sorted(cwd.rglob("*")):
        if not path.is_file() or path.is_symlink():
            continue
        rel = path.relative_to(cwd).as_posix()
        if rel.startswith("serve-") or rel.startswith(".zhuye/"):
            continue  # 诊断残留与 CLI 自身工作区配置目录
        try:
            files[rel] = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            files[rel] = "<unreadable>"
    return files


def compare_projections(base: dict, cand: dict) -> list[str]:
    """比较两侧投影,返回人类可读分歧列表(空列表 = 无分歧)。"""
    divergences: list[str] = []
    if base["rollout_types"] != cand["rollout_types"]:
        divergences.append(
            f"rollout 记录类型不一致:baseline={base['rollout_types']}"
            f" candidate={cand['rollout_types']}")
    if base["workspace_files"] != cand["workspace_files"]:
        divergences.append(
            f"工作区副作用不一致:baseline={json.dumps(base['workspace_files'], ensure_ascii=False)}"
            f" candidate={json.dumps(cand['workspace_files'], ensure_ascii=False)}")
    for needle, present in base["markers"].items():
        if present and not cand["markers"].get(needle):
            divergences.append(f"候选屏幕缺少标记:{needle}")
    for needle, present in cand["markers"].items():
        if present and not base["markers"].get(needle):
            divergences.append(f"候选新增屏幕标记:{needle}")
    for needle in base.get("forbid_screen", []):
        if cand["markers"].get(needle):
            divergences.append(f"候选屏幕出现禁止标记:{needle}")
    if cand["panicked"]:
        divergences.append("候选二进制出现 panic")
    return divergences


# ── 执行器层:驱动真实 PTY ───────────────────────────────────────────────


def run_scenario(scenario: dict, binary: Path, out_dir: Path | None = None) -> dict:
    """对单个二进制执行一个差分场景,返回语义投影。

    基线自身的健全性(标记齐备、无 panic)在本函数内校验:基线不满足
    说明场景本身坏了,属基础设施问题,应失败而非产出假分歧。
    """
    sandbox = Sandbox.create(prefix="zhuye-cli-diff-")
    try:
        # mock 脚本放沙箱根(HOME 之外),不污染工作区快照。
        script = write_mock_script(
            sandbox.home.parent / "mock-script.jsonl", scenario["script_lines"])
        env = sandbox.mock_env(script)
        screen = ""
        with TuiSession(sandbox, env=env, binary=binary) as first:
            first.wait_boot(timeout=40)
            first.type_text(scenario["prompt"])
            first.key("enter")
            first.wait_text(scenario["wait_marker"], timeout=40)
            screen = first.wait_idle(timeout=40)
            first.quit(timeout=10)
        if scenario["resume"]:
            files = sandbox.session_files()
            assert files, "恢复场景应产生会话文件"
            session_id = files[-1].stem
            with TuiSession(sandbox, args=["--resume", session_id],
                            env=env, binary=binary) as second:
                second.wait_boot(timeout=40)
                screen = second.wait_text("已恢复历史会话", timeout=30)
                screen = second.wait_idle(timeout=40)
                second.quit(timeout=10)
        for needle in scenario["expect_screen"]:
            assert needle in screen, f"基线屏幕缺少标记 {needle!r}(场景健全性)\n{screen[-600:]}"
        assert not any(needle in screen for needle in scenario["forbid_screen"]), \
            f"基线屏幕出现禁止标记(场景健全性)\n{screen[-600:]}"
        projection = {
            "rollout_types": parse_rollout_types(sandbox.session_files()),
            "workspace_files": workspace_snapshot(sandbox.cwd),
            "markers": {needle: needle in screen
                        for needle in scenario["expect_screen"] + scenario["forbid_screen"]},
            "panicked": False,
        }
        if out_dir is not None:
            target = out_dir / scenario["id"]
            target.mkdir(parents=True, exist_ok=True)
            (target / "projection.json").write_text(
                json.dumps(projection, ensure_ascii=False, indent=2), encoding="utf-8")
            (target / "screen.txt").write_text(screen, encoding="utf-8")
        return projection
    finally:
        sandbox.cleanup()


# ── pytest 入口 ──────────────────────────────────────────────────────────


class TestCompareProjections:
    """场景比较器自测:控制组零分歧,注入漂移必被抓(不依赖二进制)。"""

    def control(self) -> dict:
        return {
            "rollout_types": ["session", "message", "message"],
            "workspace_files": {},
            "markers": {"差分验证回复": True, "panicked at": False},
            "forbid_screen": ["panicked at"],
            "panicked": False,
        }

    def test_identical_projections_no_divergence(self):
        assert compare_projections(self.control(), self.control()) == []

    def test_missing_marker_detected(self):
        cand = self.control()
        cand["markers"] = {"差分验证回复": False, "panicked at": False}
        divergences = compare_projections(self.control(), cand)
        assert any("候选屏幕缺少标记:差分验证回复" in d for d in divergences)

    def test_new_marker_detected(self):
        cand = self.control()
        cand["markers"]["新标记"] = True
        divergences = compare_projections(self.control(), cand)
        assert any("候选新增屏幕标记:新标记" in d for d in divergences)

    def test_rollout_mismatch_detected(self):
        cand = self.control()
        cand["rollout_types"] = ["session", "message"]
        divergences = compare_projections(self.control(), cand)
        assert any("rollout 记录类型不一致" in d for d in divergences)

    def test_workspace_side_effect_mismatch_detected(self):
        cand = self.control()
        cand["workspace_files"] = {"diff-out.txt": "差分数据"}
        divergences = compare_projections(self.control(), cand)
        assert any("工作区副作用不一致" in d for d in divergences)

    def test_panic_detected(self):
        cand = self.control()
        cand["panicked"] = True
        divergences = compare_projections(self.control(), cand)
        assert any("候选二进制出现 panic" in d for d in divergences)


def _diff_binaries() -> tuple[Path, Path] | None:
    """读取差分二进制环境变量;未提供返回 None(纯自测模式)。"""
    baseline = os.environ.get("ZHUYE_DIFF_BASELINE")
    candidate = os.environ.get("ZHUYE_DIFF_CANDIDATE")
    if not baseline or not candidate:
        return None
    return Path(baseline), Path(candidate)


@pytest.mark.parametrize("scenario", SCENARIOS, ids=lambda s: s["id"])
def test_真实差分(scenario: dict) -> None:
    """真实差分:同一场景跑两个二进制,语义投影必须零分歧。

    未提供 ZHUYE_DIFF_BASELINE / ZHUYE_DIFF_CANDIDATE 时跳过 —— 由条件任务
    tui.contract.differential 在显式启用时执行。
    """
    binaries = _diff_binaries()
    if binaries is None:
        pytest.skip("未提供 ZHUYE_DIFF_BASELINE / ZHUYE_DIFF_CANDIDATE")
    # 子进程的 cwd 是临时沙箱,二进制路径必须先解析为绝对路径。
    baseline, candidate = (path.resolve() for path in binaries)
    for label, path in (("baseline", baseline), ("candidate", candidate)):
        if not path.is_file():
            pytest.fail(f"{label} 二进制不存在:{path}(条件任务显式启用时必须可用)")
    out_dir = Path(os.environ.get("ZHUYE_DIFF_OUTPUT", str(DEFAULT_OUTPUT)))
    base_proj = run_scenario(scenario, baseline, out_dir)
    cand_proj = run_scenario(scenario, candidate, out_dir)
    divergences = compare_projections(base_proj, cand_proj)
    assert not divergences, f"差分场景 {scenario['id']} 发现分歧:\n" + "\n".join(divergences)
