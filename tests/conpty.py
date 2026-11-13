#!/usr/bin/env python3
"""Windows ConPTY 专项冒烟:驱动真实 TUI 二进制完成启动、输入与退出。"""

from __future__ import annotations

import os
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("ZHUYE_CLI_BIN", ROOT / "target/release/zhuye-cli.exe"))


def _fail_infrastructure(message: str) -> None:
    print(f"[ERROR] {message}", file=sys.stderr)
    raise SystemExit(2)


def _read(process) -> str:
    return process.read()


def _main() -> int:
    if sys.platform != "win32":
        _fail_infrastructure("ConPTY 专项仅在 Windows 上执行")
    if not BINARY.is_file():
        _fail_infrastructure(f"缺少 release 二进制:{BINARY};先 cargo build --release -p zhuye-cli")

    try:
        import pyte
        from winpty import PtyProcess
    except ImportError as exc:
        _fail_infrastructure(f"缺少 pywinpty/pyte:{exc};先 pip install pywinpty pyte")

    with tempfile.TemporaryDirectory(prefix="zhuye-conpty-") as temp_name:
        temp = Path(temp_name)
        home = temp / "home"
        cwd = temp / "cwd"
        config = home / ".zhuye"
        config.mkdir(parents=True)
        cwd.mkdir()
        (config / "config.toml").write_text(
            "# Windows ConPTY 测试用最小配置\n", encoding="utf-8"
        )

        env = os.environ.copy()
        for name in (
            "HOME",
            "USERPROFILE",
            "HOMEDRIVE",
            "HOMEPATH",
            "APPDATA",
            "LOCALAPPDATA",
        ):
            env.pop(name, None)
        env.update(
            {
                "HOME": str(home),
                "USERPROFILE": str(home),
                "APPDATA": str(home / "AppData/Roaming"),
                "LOCALAPPDATA": str(home / "AppData/Local"),
                "ZHUYE_MODEL": "mock/mock-1",
                "REFLECT_TUI_UPDATE_CHECK": "0",
                "TERM": "xterm-256color",
            }
        )

        screen = pyte.Screen(110, 34)
        stream = pyte.ByteStream(screen)
        boot_text = ""
        process = PtyProcess.spawn(
            str(BINARY),
            dimensions=(110, 34),
            cwd=str(cwd),
            env=env,
        )
        try:
            deadline = time.monotonic() + 30
            boot_visible = False
            while time.monotonic() < deadline:
                if not process.isalive():
                    _fail_infrastructure("TUI 在启动完成前退出")
                chunk = _read(process)
                if chunk:
                    stream.feed(chunk.encode("utf-8", errors="replace"))
                    boot_text = "\n".join(screen.display)
                    # ConPTY 会在 110 列处折行,不能要求提示语在同一行连续出现。
                    boot_visible = "ZhuYe-CLI" in boot_text and "/help" in boot_text
                    if boot_visible:
                        break
                time.sleep(0.05)
            else:
                print(boot_text[-1000:], file=sys.stderr)
                _fail_infrastructure("ConPTY 30s 内未出现启动横幅关键内容")

            process.write("conpty-input-probe\r")
            deadline = time.monotonic() + 20
            input_visible = False
            while time.monotonic() < deadline:
                chunk = _read(process)
                if chunk:
                    stream.feed(chunk.encode("utf-8", errors="replace"))
                    visible = "\n".join(screen.display)
                    input_visible = "conpty-input-probe" in visible
                    if input_visible:
                        break
                time.sleep(0.05)
            if not input_visible:
                print("\n".join(screen.display)[-1000:], file=sys.stderr)
                _fail_infrastructure("输入未出现在 ConPTY 屏幕缓冲")

            process.write(chr(3))
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline and process.isalive():
                try:
                    chunk = _read(process)
                    if chunk:
                        stream.feed(chunk.encode("utf-8", errors="replace"))
                except EOFError:
                    break
                time.sleep(0.05)
            if process.isalive():
                process.terminate(force=True)
                _fail_infrastructure("Ctrl+C 后 TUI 未在 10s 内退出")
        finally:
            if process.isalive():
                process.terminate(force=True)

    print("PASS Windows ConPTY boot/input/exit smoke")
    return 0


if __name__ == "__main__":
    raise SystemExit(_main())
