#!/usr/bin/env python3
"""cli_e2e_lib.py —— TUI 端到端演练套件的共享库。

被 `cli_e2e_run.py`(单场景执行器)与 `cli_e2e_scenarios/*`(各场景)使用:

- 配置:`load_config` 读模型配置 toml(默认 ZhuYe 主仓
  `tests/live/cli_e2e.toml`:端点 / 模型 / 运行参数 / 任务与验收期望)
  并应用 `ZHUYE_E2E_*` 环境变量按字段名覆盖;
- 隔离环境:`make_env` 为单个场景生成独立的假 HOME(ZHUYE_HOME)、引擎
  配置与工作区 —— 会话 rollout / traces / 产物全落临时目录,不碰真实
  `~/.zhuye`;
- TUI 驱动:`TuiDriver` 包装 `tui_pty.TuiSession`(pty + pyte 屏幕快照),
  提供「发提示词 → 等回合收尾 → 自动应答弹层」的同步原语。断言口径是
  本仓的「屏幕可见文本 / 落盘文件内容 / 进程退出码」,与 ZhuYe 主仓
  同名套件(headless serve 的 wire 事件断言)互补;
- 检查报告:`Checks` 收集结构化检查项(pass / fail / skip),由套件入口
  逐项折成 tcase;
- mock 自检:`ZHUYE_E2E_MOCK=1` 时用内置 mock provider(`ZHUYE_MODEL=
  mock/mock-1` + `ZHUYE_MOCK_SCRIPT`)脚本化模型回复,不依赖任何端点
  即可验证场景 harness 本身(与 functional/ 的离线口径一致)。

与 ZhuYe 主仓 `cli_e2e_lib.py` 的关键差异:那边等 wire 事件
(turn_complete / plan_ready / approval_request),这边只有屏幕 ——
「回合收尾」= HUD 忙碌 spinner 消失(tui_pty.wait_idle 同源),「弹层
待应答」= 屏幕出现 overlay 锚点文本(计划批准 `[1] Auto`、工具审批
`APPROVAL NEEDED`),由轮询循环按序消化。
"""

from __future__ import annotations

import glob
import json
import os
import re
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass
from pathlib import Path

# 复用 functional/ 的 pty 设施与同目录 live_lane 的屏幕 / 落盘助手 ——
# 两个 import 面都在本仓测试分层内,不在 Python 里复写轮子。
_HERE = os.path.dirname(os.path.abspath(__file__))
_FUNCTIONAL = os.path.join(os.path.dirname(_HERE), "functional")
for _p in (_HERE, _FUNCTIONAL):
    if _p not in sys.path:
        sys.path.insert(0, _p)

from live_lane import rate_limited, session_stats  # noqa: E402
from tui_pty import TuiSession  # noqa: E402

# 工具审批 overlay 的屏幕锚点(composer 上方的审批面板)。真模型可能
# 顺手调用任何需审批的工具,不兜底应答就把回合挂到超时 —— 与上游 lib
# 的「approval_request 兜底批准」同口径。
APPROVAL_ANCHOR = "APPROVAL NEEDED"
# 计划批准 overlay 的锚点(plan_ready 打开的审批面板,[1] 批准执行)。
PLAN_ANCHOR = "[1] Auto"

# 上游限流 / 配额耗尽的屏幕锚点(与 live_lane.RATE_LIMIT_MARKERS 同源,
# 经 rate_limited() 判定)—— 限流判「跳过」而不是失败:测的是代码,不是配额。

# 会话 rollout 记录形状(zhuye_protocol::RolloutRecord,见 live_lane 注释):
# - `{"type":"token_count","usage":{...},"model":"<group>/<id>"}`;
# - `{"type":"message","role":"assistant","content":...}`。

# 标量配置:toml 字段 → 环境变量覆盖名(前缀 ZHUYE_E2E_)。
_SCALAR_KEYS = (
    "BASE_URL",
    "API_KEY",
    "MODEL",
    "ENDPOINT",
    "CONTEXT_WINDOW",
    "EFFORT",
    "PHASE_TIMEOUT",
    "EXPECT_ARTIFACT",
    "EXPECT_COMMIT",
)


# ── 配置 ────────────────────────────────────────────────────────────────────


def scalars(cfg: dict) -> dict:
    """把 toml 的端点 / 运行参数 / 任务期望摊平成字符串字典。"""
    ep = cfg.get("endpoint") or {}
    run = cfg.get("run") or {}
    task = cfg.get("task") or {}
    return {
        "BASE_URL": str(ep.get("base_url") or ""),
        "API_KEY": str(ep.get("api_key") or ""),
        "MODEL": str(ep.get("model") or ""),
        "ENDPOINT": str(ep.get("endpoint") or "openai"),
        "CONTEXT_WINDOW": str(ep.get("context_window") or ""),
        "EFFORT": str(run.get("effort") or "high"),
        "PHASE_TIMEOUT": str(run.get("phase_timeout") or 1200),
        "EXPECT_ARTIFACT": str(task.get("expect_artifact") or ""),
        "EXPECT_COMMIT": str(task.get("expect_commit_subject") or ""),
    }


def load_config(toml_path: str, env: dict | None = None) -> dict:
    """读取模型配置并应用字段级环境变量覆盖(环境变量优先于 toml)。"""
    with open(toml_path, "rb") as f:
        cfg = tomllib.load(f)
    env = os.environ if env is None else env
    flat = scalars(cfg)
    for key in _SCALAR_KEYS:
        override = env.get(f"ZHUYE_E2E_{key}")
        if override:
            flat[key] = override
    cfg["_scalars"] = flat
    return cfg


# ── 隔离环境 ────────────────────────────────────────────────────────────────


def engine_config_toml(
    scalars_: dict,
    *,
    window: int | None = None,
    plugins: list[str] | None = None,
    extra_toml: str = "",
) -> str:
    """生成隔离 HOME 里的引擎配置(zhuye-config v2 模型组格式)。

    最小独立面(不继承真实 ~/.zhuye 的插件 / MCP / hooks),演练结果只
    取决于本配置 + 引擎默认。`extra_toml` 追加在末尾(场景专属片段)。
    mock 自检模式下 ZHUYE_MODEL=mock/mock-1 优先于配置里的 model 字段
    (zhuye-config 的解析优先级:env ZHUYE_MODEL > 组 model)。
    """
    q = json.dumps
    lines = [
        "# 由 cli_e2e 套件生成 —— TUI 端到端演练的独立引擎配置。",
        "[active]",
        'group = "e2e"',
        "",
        "[[model_group]]",
        'id = "e2e"',
        f"endpoint = {q(scalars_.get('ENDPOINT') or 'openai')}",
        f"api_key = {q(scalars_.get('API_KEY', ''))}",
        f"base_url = {q(scalars_.get('BASE_URL', ''))}",
        f"model = {q(scalars_.get('MODEL', ''))}",
        "responses_api = false",
        # 慢思考的流式响应不能被默认 HTTP 超时腰斩(与 live_lane 同口径)。
        "timeout_secs = 300",
        "credentials = []",
        "",
    ]
    window = window if window is not None else (
        int(scalars_["CONTEXT_WINDOW"]) if scalars_.get("CONTEXT_WINDOW", "").isdigit() else None
    )
    if window is not None and scalars_.get("MODEL"):
        lines += [
            "[context_windows]",
            f"{q(scalars_['MODEL'])} = {int(window)}",
            "",
        ]
    lines += [
        "[sandbox]",
        "os_level = true",
        "writable_dirs = []",
        "# 任务夹具虽纯本地,但不人为限制 agent 的合法依赖。",
        "allow_network = true",
        "",
        "[plugins]",
        "enabled_plugins = [" + ", ".join(q(p) for p in (plugins or [])) + "]",
        "disabled_plugins = []",
        "",
    ]
    if extra_toml:
        lines += [extra_toml.rstrip("\n"), ""]
    return "\n".join(lines)


@dataclass
class Env:
    """单个场景的隔离环境:假 HOME + 引擎配置 + 工作区 + 子进程 env。"""

    work: str
    home: str
    ws: str
    zhuye_home: str
    proc_env: dict

    # ── 观测面:会话 rollout / traces(都在假 HOME 下)─────────────────────

    def session_files(self) -> list[str]:
        """隔离 HOME 下的会话 JSONL 列表(与 live_lane 同口径)。"""
        pat = os.path.join(self.zhuye_home, "sessions", "*", "*", "*", "*.jsonl")
        return sorted(glob.glob(pat, recursive=True))

    def session_records(self) -> list[dict]:
        """全部会话 rollout 记录(坏行跳过,统计不因单行失败)。"""
        out: list[dict] = []
        for path in self.session_files():
            with open(path, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            out.append(json.loads(line))
                        except json.JSONDecodeError:
                            pass
        return out

    def assistant_texts(self) -> list[str]:
        """已落盘的全部 assistant 消息文本(轮次收尾判据 / 回放断言用)。"""
        texts = []
        for rec in self.session_records():
            if rec.get("type") == "message" and rec.get("role") == "assistant":
                content = rec.get("content")
                if isinstance(content, str):
                    texts.append(content)
                elif isinstance(content, list):
                    texts.append("".join(
                        b.get("text") or ""
                        for b in content
                        if isinstance(b, dict) and b.get("type") == "text"
                    ))
        return texts

    def stats(self) -> dict:
        """会话统计(token 用量 / 实际服务模型 / assistant 条数)。"""
        return session_stats([Path(p) for p in self.session_files()])


def make_env(
    work: str,
    cfg: dict,
    *,
    extra_toml: str = "",
    env_extra: dict | None = None,
    mock_script: list[str] | None = None,
    window: int | None = None,
    plugins: list[str] | None = None,
) -> Env:
    """在 `work` 下生成隔离环境(假 HOME + 配置 + 工作区 + git 身份)。"""
    home = os.path.join(work, "home")
    ws = os.path.join(work, "ws")
    zhuye_home = os.path.join(home, ".zhuye")
    os.makedirs(zhuye_home, exist_ok=True)
    os.makedirs(ws, exist_ok=True)
    with open(os.path.join(zhuye_home, "config.toml"), "w", encoding="utf-8") as f:
        f.write(engine_config_toml(
            cfg["_scalars"], window=window, plugins=plugins, extra_toml=extra_toml,
        ))
    # git 身份经假 HOME 的 .gitconfig 提供(任务场景的 git init/commit 全走它)。
    with open(os.path.join(home, ".gitconfig"), "w", encoding="utf-8") as f:
        f.write(
            "[user]\n"
            "    name = zhuye-e2e\n"
            "    email = zhuye-e2e@example.com\n"
            "[commit]\n"
            "    gpgsign = false\n"
        )
    proc_env = dict(os.environ)
    proc_env["HOME"] = home
    # ZHUYE_HOME 指向数据目录本身(其下直接放 config.toml,不拼 .zhuye)。
    proc_env["ZHUYE_HOME"] = zhuye_home
    # 不让宿主的引擎环境变量渗进演练(与 tests/lib.sh make_isolated_env 同口径)。
    for name in list(proc_env):
        if name.startswith("ZHUYE_") and name != "ZHUYE_HOME":
            del proc_env[name]
    for key in ("OPENAI_API_KEY", "ANTHROPIC_API_KEY", "GEMINI_API_KEY", "OLLAMA_API_KEY"):
        proc_env.pop(key, None)
    # 屏蔽启动期 GitHub releases 探测 —— 离线/内网环境不卡启动。
    proc_env["ZHUYE_TUI_UPDATE_CHECK"] = "0"
    # crossterm 在 TERM=dumb / 缺省下不渲染(non-ANSI terminal)。
    if not proc_env.get("TERM") or proc_env["TERM"] == "dumb":
        proc_env["TERM"] = "xterm-256color"
    if env_extra:
        proc_env.update(env_extra)
    if mock_script is not None:
        script_path = os.path.join(work, "mock-script.jsonl")
        with open(script_path, "w", encoding="utf-8") as f:
            for reply in mock_script:
                f.write(reply if isinstance(reply, str) else json.dumps(reply, ensure_ascii=False))
                f.write("\n")
        proc_env["ZHUYE_MODEL"] = "mock/mock-1"
        proc_env["ZHUYE_MOCK_SCRIPT"] = script_path
    return Env(work=work, home=home, ws=ws, zhuye_home=zhuye_home, proc_env=proc_env)


class _EnvShim:
    """把 `Env` 适配成 `TuiSession(sandbox=...)` 需要的最小接口。

    TuiSession 只用 `sandbox.cwd` 与 `sandbox.env(env)`;这里**不能**走
    `Sandbox.env()` —— 它以 os.environ 为底,宿主凭据会渗进隔离 env。
    本垫片直接返回 make_env 产出的已清洗 env(HOME / 凭据都已处理)。
    """

    def __init__(self, env: Env):
        self.cwd = env.ws
        self._env = env.proc_env

    def env(self, extra: dict | None = None) -> dict:
        merged = dict(self._env)
        if extra:
            merged.update(extra)
        return merged


# ── 检查报告 ────────────────────────────────────────────────────────────────


class Checks:
    """结构化检查项。fail 任一 → 场景 FAIL;全部 skip → 场景 SKIP。"""

    def __init__(self) -> None:
        self.items: list[dict] = []

    def add(self, name: str, ok: bool, detail: str = "") -> bool:
        self.items.append({"name": name, "status": "pass" if ok else "fail", "detail": detail})
        return ok

    def skip(self, name: str, reason: str) -> None:
        self.items.append({"name": name, "status": "skip", "detail": reason})

    @property
    def status(self) -> str:
        if any(i["status"] == "fail" for i in self.items):
            return "FAIL"
        if self.items and all(i["status"] == "skip" for i in self.items):
            return "SKIP"
        return "PASS"


# ── TUI 驱动 ────────────────────────────────────────────────────────────────


class WaitTimeout(Exception):
    """等待条件超时。"""


class TuiDied(Exception):
    """TUI 进程意外退出。"""


class RateLimited(Exception):
    """上游限流 / 配额耗尽(屏幕锚点命中)—— 整场景判 SKIP 的信号。"""


# HUD 忙碌 spinner 帧(tui_pty._BUSY_RE 同源):忙碌时状态行以
# `<帧> ` 开头,空闲时无 —— 与 wait_idle 的判定一致。
_BUSY_RE = re.compile("[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] ")


def screen_text(s: TuiSession) -> str:
    """读屏幕归一化文本;pyte 偶发解析异常**不**带崩用例(与 live_lane.safe_text
    同口径:读不到就当这一轮没变化,下一轮继续)。"""
    try:
        return s.normalized()
    except Exception:  # noqa: BLE001 - 读屏尽力而为
        return ""


class TuiDriver:
    """一个 TUI 进程的同步驱动:发提示词 → 等回合收尾 → 自动应答弹层。

    处理器签名统一为 `(driver, screen) -> bool`(True = 已消费):等待
    循环每 0.5s 读一次屏,先让处理器消化需要即时应答的 overlay(计划
    批准 / 工具审批),再判空闲。未被处理器消费的审批 overlay 由
    `_auto_answers` 兜底放行 —— 真模型可能顺手调用任何需审批的工具,
    不兜底就把回合挂到超时(与上游 lib 的 approval_request 兜底批准同口径)。
    """

    def __init__(self, session: TuiSession, env: "Env"):
        self.s = session
        self.env = env
        self.approvals_seen: list[str] = []

    def __enter__(self) -> "TuiDriver":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        # 异常路径也要收掉 pty 进程,不留孤儿 TUI(tui_pty 同口径)。
        if exc is not None:
            try:
                self.s._proc  # noqa: B018 - 存在性检查,失败随它去
                os.killpg(os.getpgid(self.s._proc.pid), 9)
            except Exception:  # noqa: BLE001 - 收尾尽力而为
                pass
        self.quit()

    @property
    def alive(self) -> bool:
        return self.s.alive()

    def send(self, text: str) -> None:
        """整段写入提示词并回车(等价真实粘贴/快速输入)。"""
        self.s.type_text(text)
        self.s.key("enter")

    def _auto_answers(self, screen: str) -> bool:
        """兜底应答:审批面板出现 → 输入 y 放行。返回 True 表示已消费。

        计划批准 overlay **不**兜底 —— 批准与否是场景语义的一部分,
        由场景自带处理器(先于本兜底消费)决定。
        """
        if APPROVAL_ANCHOR in screen:
            self.approvals_seen.append(screen)
            self.s.type_text("y")
            time.sleep(1.0)
            return True
        return False

    def wait_idle(
        self,
        timeout: float,
        respond=None,
        what: str = "回合收尾",
    ) -> str:
        """等回合收尾(spinner 消失)并返回当时屏幕文本。

        与 tui_pty.wait_idle 的差异:轮询途中先跑 `respond` 处理器与
        兜底应答(消化 overlay),并监视上游限流与进程死亡。TUI 在
        幂等状态下(如刚启动、弹层打开)也满足「无 spinner」,所以
        「等回合」必须先 `send` 再 `wait_idle`,与 wire 侧 turn() 的
        「先投递再等终态」同序。
        """
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise WaitTimeout(f"等待「{what}」超时({timeout:.0f}s)")
            screen = screen_text(self.s)
            consumed = False
            if respond:
                for handler in respond:
                    if handler(self, screen):
                        consumed = True
                        break
            if not consumed:
                consumed = self._auto_answers(screen)
            if rate_limited(screen):
                raise RateLimited(screen[:200])
            if not consumed and not _BUSY_RE.search(screen):
                return screen
            if not self.s.alive():
                raise TuiDied("TUI 进程在等待回合收尾时退出")
            time.sleep(0.5)

    def wait_screen(self, needle: str, timeout: float) -> str:
        """等屏幕出现 `needle`(归一化子串匹配),返回当时屏幕文本。"""
        deadline = time.monotonic() + timeout
        while True:
            screen = screen_text(self.s)
            if needle in screen:
                return screen
            if rate_limited(screen):
                raise RateLimited(screen[:200])
            if not self.s.alive():
                raise TuiDied(f"TUI 进程在等待「{needle}」时退出")
            if time.monotonic() > deadline:
                raise WaitTimeout(f"等待「{needle}」超时({timeout:.0f}s)")
            time.sleep(0.5)

    def turn(
        self,
        text: str,
        *,
        timeout: float,
        respond=None,
    ) -> str:
        """投递一条提示词并等**回合真正收尾**,返回收尾后的屏幕文本。

        收尾信号是双重的:忙碌 spinner 消失 **且** 落盘 assistant 消息数
        增加 —— 只看 spinner 会撞上「提交还没起回合就轮询到空闲」的竞态
        (mock 回复近即时,spinner 存活 <1s;与 practice.py 的收尾判据同
        口径)。收尾后再静置 0.8s:历史行定型与落盘的先后有毫秒级窗口,
        返回的屏幕文本要能直接做子串断言。
        """
        baseline = len(self.env.assistant_texts())
        self.send(text)
        # 提交回显守卫:提示词若被吞(composer 未就绪等),快速失败而不是
        # 拖到阶段超时 —— 回显出现在 composer 预览或历史 ▶ 行,归一化后
        # 必含提示词前缀。
        echo_needle = " ".join(text[:80].split())
        try:
            self.wait_screen(echo_needle, timeout=min(timeout, 45.0))
        except WaitTimeout as e:
            raise WaitTimeout(f"提示词未被提交(输入回显未出现):{e}") from e
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                tail = "\n".join(self.s.display_lines()[-10:])
                raise WaitTimeout(
                    f"等待回合收尾超时({timeout:.0f}s;基线 {baseline} 条,已落盘 "
                    f"{len(self.env.assistant_texts())} 条)\n--- 屏幕尾部 ---\n{tail}"
                )
            screen = screen_text(self.s)
            consumed = False
            if respond:
                for handler in respond:
                    if handler(self, screen):
                        consumed = True
                        break
            if not consumed:
                consumed = self._auto_answers(screen)
            if rate_limited(screen):
                raise RateLimited(screen[:200])
            if not self.s.alive():
                raise TuiDied("TUI 进程在等待回合收尾时退出")
            if (
                not consumed
                and not _BUSY_RE.search(screen)
                and len(self.env.assistant_texts()) > baseline
            ):
                time.sleep(0.8)
                return screen_text(self.s)
            time.sleep(0.4)

    def quit(self) -> None:
        """礼貌退出(双击 Ctrl-C,SIGTERM 兜底)。"""
        self.s.close()


# ── 场景与执行上下文 ────────────────────────────────────────────────────────


class Scenario:
    """一个端到端场景的注册信息。

    - `run(cx)`:场景主体,经 `cx.env()` 生成隔离环境、`cx.tui(env)` 起
      TUI 驱动、`cx.checks.add/skip` 登记检查项;
    - `needs_model=True`:本质依赖真实模型(真实任务的全链路:出 plan /
      批准执行 / git 提交),mock 自检模式下整场景 SKIP;
    - `mock_script(params)`:mock 自检用的脚本化模型回复(每次模型调用
      一行,与 functional/ 的 ZHUYE_MOCK_SCRIPT 行格式一致);`needs_model
      =False` 的场景必须提供,否则自检跳过该场景;
    - `defaults`:可被 toml `[scenarios.<name>]` 覆盖的参数默认值。
    """

    def __init__(
        self,
        name: str,
        title: str,
        run,
        *,
        needs_model: bool = False,
        defaults: dict | None = None,
        mock_script=None,
    ):
        self.name = name
        self.title = title
        self.run = run
        self.needs_model = needs_model
        self.defaults = defaults or {}
        self.mock_script = mock_script


class Ctx:
    """单个场景的执行上下文:参数、检查表、TUI 生命周期、观测面。"""

    def __init__(self, scenario: Scenario, cfg: dict, params: dict, work: str, bin: str, mock: bool):
        self.scenario = scenario
        self.cfg = cfg
        self.params = params
        self.work = work
        self.bin = bin
        self.mock = mock
        self.checks = Checks()
        self.tuis: list[TuiDriver] = []
        self.metrics: dict = {}
        os.makedirs(work, exist_ok=True)

    def p(self, key: str, default=None):
        """取场景参数;空串 / None 视为「未设置」回落 default —— 与 shell 的
        `ZHUYE_E2E_*` 覆盖语义一致(`if override` 把空串当未设),否则
        defaults 里的空串哨兵会顶掉场景内置默认值,发出空提示词。"""
        value = self.params.get(key)
        if value is None or value == "":
            return default
        return value

    def timeout(self, default: float = 600.0) -> float:
        """阶段超时:场景参数 `phase_timeout` > 全局 run.phase_timeout。"""
        value = self.params.get("phase_timeout")
        if value:
            return float(value)
        return float(self.cfg["_scalars"].get("PHASE_TIMEOUT") or default)

    def effort(self) -> str:
        """思考强度:场景参数 `effort` > 全局 run.effort(TUI `/effort` 可设)。"""
        return str(self.params.get("effort") or self.cfg["_scalars"].get("EFFORT") or "high")

    def env(self, *, extra_toml: str = "", env_extra: dict | None = None,
            window: int | None = None, plugins: list[str] | None = None,
            setup=None, mock_script: list[str] | None = None) -> Env:
        """生成本场景的隔离环境(`setup(ws)` 可布置工作区夹具)。

        `mock_script` 传脚本行列表可覆盖场景注册的默认脚本(同一场景内
        需要多段不同回复时用,如 session_resume 的两段);None = 用场景
        注册的 `mock_script(params)`。仅 mock 自检模式消费。
        """
        script = None
        if self.mock:
            script = mock_script if mock_script is not None else (
                self.scenario.mock_script(self.params) if self.scenario.mock_script else []
            )
        env = make_env(
            self.work,
            self.cfg,
            extra_toml=extra_toml,
            env_extra=env_extra,
            mock_script=script,
            window=window,
            plugins=plugins,
        )
        if setup:
            setup(env.ws)
        return env

    def tui(self, env: Env, *, args: list[str] | None = None, label: str = "main",
            cols: int = 130, rows: int = 44) -> TuiDriver:
        """在隔离环境里起一个 TUI 进程并等到稳定首屏。"""
        driver = TuiDriver(
            TuiSession(_EnvShim(env), args=args, env=None, cols=cols, rows=rows,
                       binary=Path(self.bin)),
            env,
        )
        self.tuis.append(driver)
        try:
            driver.s.wait_boot(timeout=90.0)
            # boot 锚点出现后 composer 可能还没就绪(实测:立即输入会被吞,
            # 与 practice.py / test_workflow_live.py 的 wait_stable 惯例对齐)。
            driver.s.wait_stable(max_wait=12.0, quiet=0.4)
        except AssertionError as e:
            raise TuiDied(f"TUI 启动未到稳定首屏:{e}") from e
        return driver

    def set_effort(self, driver: TuiDriver, level: str) -> None:
        """经 `/effort` 设置思考强度(best-effort:发完稍候,不锚定回显)。"""
        driver.send(f"/effort {level}")
        time.sleep(2.0)

    def close_all(self) -> None:
        for d in self.tuis:
            try:
                d.s.close()
            except Exception:  # noqa: BLE001 - 收尾尽力而为
                pass
        self.tuis = []

    # ── 工作区 / git 观测面 ────────────────────────────────────────────────

    def write_file(self, ws: str, rel: str, content: str) -> str:
        path = os.path.join(ws, rel)
        os.makedirs(os.path.dirname(path) or ws, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return path

    def exists(self, ws: str, rel: str) -> bool:
        return os.path.exists(os.path.join(ws, rel))

    def commit_subjects(self, ws: str) -> list[str]:
        """工作区 git 仓库的全部提交主题行(任务场景的验收断言用)。"""
        proc = subprocess.run(
            ["git", "log", "--format=%s"],
            cwd=ws,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if proc.returncode != 0:
            return []
        return [line for line in proc.stdout.splitlines() if line.strip()]


def skip_all(cx: Ctx, names: list[str], reason: str) -> None:
    """把一组检查项标为 skip。

    真模型自由发挥:未必按提示词调用指定工具 / 走预期路径。这种
    「模型没按预期走」判**跳过**而非失败 —— TUI 链路本身没问题,
    缺的是模型配合(确定性自检由 mock 模式覆盖)。
    """
    for name in names:
        cx.checks.skip(name, reason)


def wait_for_file(env: Env, rel: str, timeout: float, driver: TuiDriver | None = None,
                  respond=None) -> bool:
    """等工作区产物出现;期间继续驱动 TUI(消化审批 overlay / 监视限流)。

    真模型的开发阶段可能多次请求审批,等文件不能干等 —— `driver` 给了
    就边等边泵。返回文件是否在时限内出现。
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if os.path.exists(os.path.join(env.ws, rel)):
            return True
        if driver is not None:
            if not driver.s.alive():
                raise TuiDied("TUI 进程在等待产物时退出")
            screen = screen_text(driver.s)
            consumed = False
            if respond:
                for handler in respond:
                    if handler(driver, screen):
                        consumed = True
                        break
            if not consumed:
                consumed = driver._auto_answers(screen)
            if rate_limited(screen):
                raise RateLimited(screen[:200])
            time.sleep(1.0)
        else:
            time.sleep(0.5)
    return os.path.exists(os.path.join(env.ws, rel))
