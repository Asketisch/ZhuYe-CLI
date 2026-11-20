"""G 组 —— 插件面:启动通知、slash 命令注入、`/plugin` overlay、门控插件。

插件能力由 submodule 的 `reflect-plugin` 提供,本仓 TUI 侧只消费四处:
1. **启动通知**:bootstrap 挂载成功后 `PluginLoaded` 事件经 lifecycle 通道
   进通知流(scrollback 出现 `Plugin 'demo@inline' loaded …`);
2. **slash 命令注入**:插件命令快照注入 composer,输入 `/demo` 弹窗列出
   `demo:hello`;提交 `/demo:hello` 不被「未知命令」校验拦截;
3. **`/plugin` overlay**:展示真实已装插件(id / version / enabled 标志);
4. **门控型 bundled 插件**(上游 8da90e5 基建,装配接线在 bootstrap.rs):
   web-search / notebook 默认开,`[plugins].disabled_plugins` 显式禁用 =
   工具不进注册表(区别于 `[active].disallowed_tools` 的可见性过滤)。
   判定单一事实源是引擎 `zhuye_plugin::bundled_plugin_enabled`。

**前置**:submodule 构建出的 `reflect` CLI(用于安装示例插件)。
缺失则整组 SKIP(不算失败)—— 插件 e2e 是增强项,不阻塞基础回归。
门控用例(G5/G6)只依赖本仓二进制 + mock provider,不依赖示例插件。

离线(`ZHUYE_MODEL=mock/mock-1`,零网络)。
"""

from __future__ import annotations

import os
import subprocess
import time
from pathlib import Path

import pytest

from tui_pty import ROOT

# submodule 构建出的 reflect-cli(bin 名是 `zhuye`(烛夜更名后))。
_CLI_CANDIDATES = [
    ROOT / "zhuye-agent" / "target" / "release" / "zhuye",
    ROOT / "zhuye-agent" / "target" / "debug" / "zhuye",
]
_EXAMPLE_PLUGIN = ROOT / "zhuye-agent" / "examples" / "plugin-demo"


def _find_cli() -> Path | None:
    for candidate in _CLI_CANDIDATES:
        if candidate.exists() and os.access(candidate, os.X_OK):
            return candidate
    return None


@pytest.fixture(scope="module")
def plugin_home(tmp_path_factory) -> str:
    """临时 HOME + 用真实 CLI 安装 / 启用示例插件(写真实表与 config)。"""
    cli = _find_cli()
    if cli is None:
        pytest.skip(
            "submodule 的 reflect CLI 未构建(需 cargo build -p reflect-cli);插件组跳过"
        )
    if not _EXAMPLE_PLUGIN.is_dir():
        pytest.skip(f"示例插件不存在:{_EXAMPLE_PLUGIN}")

    home = tmp_path_factory.mktemp("plugin-home")
    env = {**os.environ, "HOME": str(home)}
    for args in (
        ["plugin", "install", str(_EXAMPLE_PLUGIN)],
        ["plugin", "enable", "demo@inline"],
    ):
        r = subprocess.run(
            [str(cli), *args],
            env=env,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if r.returncode != 0:
            pytest.skip(f"示例插件安装失败({args}): {r.stdout}{r.stderr}")
    return str(home)


def _plugin_env(home: str) -> dict[str, str]:
    return {
        "HOME": home,
        "ZHUYE_MODEL": "mock/mock-1",
        "REFLECT_TUI_UPDATE_CHECK": "0",
    }


def test_plugin_loaded_notice(plugin_home, sandbox) -> None:
    """G1 启动通知:scrollback 出现 `Plugin 'demo@inline' loaded`。"""
    from tui_pty import TuiSession

    env = _plugin_env(plugin_home)
    # 让 sandbox 的 cwd 也用 plugin_home(插件在 HOME 下)。
    with TuiSession(sandbox, env=env, cols=120, rows=40) as s:
        # 插件挂载(含 MCP server 连接)会推迟首帧渲染,必须按内容轮询。
        deadline = time.monotonic() + 40
        txt = ""
        while time.monotonic() < deadline:
            txt = s.normalized()
            if "Plugin 'demo@inline' loaded" in txt:
                break
            time.sleep(0.4)
        assert "Plugin 'demo@inline' loaded" in txt, (
            f"应出现插件加载通知:\n{txt[-600:]}"
        )


def test_plugin_slash_command_is_listed(plugin_home, sandbox) -> None:
    """G2 slash 弹窗列出插件命令 `demo:hello`。"""
    from tui_pty import TuiSession

    env = _plugin_env(plugin_home)
    with TuiSession(sandbox, env=env, cols=120, rows=40) as s:
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            if "for commands" in s.normalized():
                break
            time.sleep(0.4)
        # 分两段输入,避免整段写入走 paste-burst 而不弹命令窗(见 COVERAGE.md D12)。
        s.type_text("/")
        s._pump(settle=0.8, max_wait=1.2)
        s.type_text("demo")
        s._pump(settle=1.0, max_wait=1.5)
        txt = s.normalized()
        assert "demo:hello" in txt, f"命令弹窗应列出插件命令:\n{txt[-800:]}"


def test_plugin_slash_command_passes_validation(
    plugin_home, sandbox, tmp_path: Path
) -> None:
    """G3 插件命令提交不被「未知命令」校验拦下。"""
    from tui_pty import TuiSession, text_reply

    env = _plugin_env(plugin_home)
    # v0.1.8 起脚本耗尽的长回退文案会把 `▶ /demo:hello` 用户行顶出屏幕
    # 窗口(见 test_slash S6 注释);配确定性短脚本让用户行常驻。
    script = tmp_path / "g3-mock.jsonl"
    script.write_text("\n".join([text_reply("ok")] * 6) + "\n")
    env["ZHUYE_MOCK_SCRIPT"] = str(script)
    with TuiSession(sandbox, env=env, cols=120, rows=40) as s:
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            if "for commands" in s.normalized():
                break
            time.sleep(0.4)
        # 分两段输入:整段写入会走 paste-burst,偶发丢字符(见 COVERAGE.md D12)。
        s.type_text("/")
        s._pump(settle=0.8, max_wait=1.2)
        s.type_text("demo:hello")
        s._pump(settle=1.0, max_wait=1.5)
        s.key("enter")
        s._pump(settle=1.5, max_wait=3.0)
        txt = s.normalized()
        assert "Unrecognized" not in txt, (
            f"插件命令不应被未知命令校验拦截:\n{txt[-600:]}"
        )
        assert "/demo:hello" in txt, f"插件命令应上屏:\n{txt[-600:]}"


def test_plugin_overlay_lists_installed_plugin(plugin_home, sandbox) -> None:
    """G4 `/plugin` overlay 展示真实已装插件(id / version / enabled)。"""
    from tui_pty import TuiSession

    env = _plugin_env(plugin_home)
    with TuiSession(sandbox, env=env, cols=120, rows=40) as s:
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            if "for commands" in s.normalized():
                break
            time.sleep(0.4)
        # 分两段输入:整段写入会走 paste-burst,偶发丢字符(见 COVERAGE.md D12)。
        s.type_text("/")
        s._pump(settle=0.8, max_wait=1.2)
        s.type_text("plugin")
        s._pump(settle=1.0, max_wait=1.5)
        s.key("enter")
        deadline = time.monotonic() + 20
        txt = ""
        while time.monotonic() < deadline:
            txt = s.normalized()
            if "Installed Plugins" in txt:
                break
            time.sleep(0.4)
        for token in ("Installed Plugins", "demo@inline", "v0.1.0", "enabled"):
            assert token in txt, f"overlay 应含 {token}:\n{txt[-800:]}"


# ── 门控型 bundled 插件(上游 8da90e5 基建;装配接线在本仓 bootstrap.rs)──
# 观察面:mock 回合里让模型调 `tool_search` / 直调工具,断言屏幕可见的
# 工具输出文案(tool_search 命中行格式 `{name} [builtin]: {desc}`;未注册
# 直调的引擎报错为 `tool not found: {name}`)。全程零网络。


def _write_sandbox_config(sandbox, body: str) -> None:
    """覆写沙箱 HOME 的 config.toml(保持「文件存在」前提,见 Sandbox.create)。"""
    (sandbox.home / ".zhuye" / "config.toml").write_text(body, encoding="utf-8")


def test_gated_plugin_default_on_registers_web_search(tui, mock_script) -> None:
    """G5 门控默认态零变化:空 `[plugins]` 下 web_search 照常注册(金测试)。

    model 调 `tool_search` 查 "web_search",命中输出行 `web_search [builtin]: …`
    即注册面存在 —— 「默认开 + 升级零影响」的对外可见口径。
    """
    from tui_pty import text_reply, tool_call

    script = mock_script(
        [
            tool_call("tool_search", {"query": "web_search"}),
            text_reply("查询完成"),
        ]
    )
    with tui(script=script) as s:
        s.wait_boot(timeout=30)
        s.type_text("search your tools")
        s.key("enter")
        txt = s.wait_text("✓ tool_search", timeout=30)
        assert "web_search [builtin]" in txt, (
            f"tool_search 应命中 web_search [builtin]:\n{txt[-600:]}"
        )


def test_gated_plugin_disabled_unregisters_web_search(tui, mock_script, sandbox) -> None:
    """G6 `[plugins].disabled_plugins = ["web-search"]` = 注册门控(非可见性过滤)。

    同一 tool_search 查询不再命中(`No tools matched`);直调 web_search 报
    `tool not found`(未注册错误,区别于「已注册但缺 key」的执行错误)。
    插件级粒度:notebook 等其余门控项不受影响。
    """
    from tui_pty import text_reply, tool_call

    _write_sandbox_config(
        sandbox,
        '# 测试配置:显式禁用 web-search 门控插件。\n'
        '[plugins]\n'
        'disabled_plugins = ["web-search"]\n',
    )
    script = mock_script(
        [
            tool_call("tool_search", {"query": "web_search"}),
            tool_call("web_search", {"query": "rust async"}),
            text_reply("查询完成"),
        ]
    )
    with tui(script=script) as s:
        s.wait_boot(timeout=30)
        s.type_text("search your tools")
        s.key("enter")
        txt = s.wait_text("✓ tool_search", timeout=30)
        assert "No tools matched" in txt, f"禁用后 tool_search 不应命中:\n{txt[-600:]}"
        assert "web_search [builtin]" not in txt, f"禁用后不应再命中 web_search:\n{txt[-600:]}"
        # 直调被禁用的工具 → 未注册错误(tool not found),而非执行错误。
        txt2 = s.wait_text("✗ web_search", timeout=30)
        assert "tool not found" in txt2, f"禁用后直调应报未注册错误:\n{txt2[-600:]}"
