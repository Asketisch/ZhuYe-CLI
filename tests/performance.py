#!/usr/bin/env python3
"""TUI 性能基线:真实 release 二进制 + PTY 的启动、长历史与输出洪峰。"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests/functional"))

from tui_pty import Sandbox, TuiSession, text_reply, tool_call  # noqa: E402


def _rss_kb(pid: int) -> int:
    result = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(pid)],
        capture_output=True,
        text=True,
        check=True,
    )
    return int(result.stdout.strip() or "0")


def _seed_large_session(sandbox: Sandbox, count: int) -> None:
    base = sandbox.home / ".zhuye/sessions/2026/01/01"
    base.mkdir(parents=True, exist_ok=True)
    session = base / "00000000-0000-4000-8000-000000000001.jsonl"
    records = [
        json.dumps(
            {
                "type": "session_meta",
                "session_id": "00000000-0000-4000-8000-000000000001",
                "model": "mock/mock-1",
                "started_at": "2026-01-01T00:00:00Z",
            },
            ensure_ascii=False,
        )
    ]
    for index in range(count):
        records.append(
            json.dumps(
                {
                    "type": "message",
                    "turn_id": "11111111-2222-4333-8444-555555555555",
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": f"performance history message {index}: " + "x" * 180,
                        }
                    ],
                },
                ensure_ascii=False,
            )
        )
    session.write_text("\n".join(records) + "\n", encoding="utf-8")


def _startup(sandbox: Sandbox, args: list[str] | None = None) -> tuple[float, int]:
    started = time.monotonic()
    session = TuiSession(
        sandbox,
        args=args,
        env=sandbox.mock_env(),
        cols=110,
        rows=34,
    )
    try:
        session.wait_boot(timeout=float(os.environ.get("TUI_STARTUP_BUDGET", "30")))
        elapsed = time.monotonic() - started
        return elapsed, _rss_kb(session._proc.pid)
    finally:
        session.close()


def main() -> int:
    binary = Path(os.environ.get("ZHUYE_CLI_BIN", ROOT / "target/release/zhuye-cli"))
    if not binary.is_file():
        print("[ERROR] 缺少 release 二进制;先 cargo build --release -p zhuye-cli", file=sys.stderr)
        return 2
    if sys.platform not in ("darwin", "linux"):
        print("[ERROR] 性能基线当前使用 Unix PTY,仅在 Linux/macOS 执行", file=sys.stderr)
        return 2

    report: dict[str, object] = {}
    with tempfile.TemporaryDirectory(prefix="zhuye-perf-") as temp_name:
        sandbox = Sandbox.create(str(Path(temp_name) / "root"))
        try:
            elapsed, rss = _startup(sandbox)
            report["startup"] = {"seconds": round(elapsed, 3), "rss_kb": rss}
            if elapsed > 15:
                raise AssertionError(f"冷启动超过 15s:{elapsed:.3f}s")
            if rss > 512 * 1024:
                raise AssertionError(f"冷启动 RSS 超过 512MiB:{rss}KiB")

            _seed_large_session(sandbox, 500)
            elapsed, rss = _startup(sandbox, ["-c"])
            report["long_history"] = {
                "messages": 500,
                "seconds": round(elapsed, 3),
                "rss_kb": rss,
            }
            if elapsed > 30:
                raise AssertionError(f"500 条历史恢复超过 30s:{elapsed:.3f}s")
            if rss > 512 * 1024:
                raise AssertionError(f"长历史 RSS 超过 512MiB:{rss}KiB")

            payload = "z" * 2500
            script = sandbox.cwd / "flood.jsonl"
            script.write_text(
                tool_call("echo", {"text": payload}) + "\n" + text_reply("PERF_FLOOD_DONE") + "\n",
                encoding="utf-8",
            )
            started = time.monotonic()
            session = TuiSession(
                sandbox,
                env=sandbox.mock_env(script),
                cols=110,
                rows=34,
            )
            try:
                session.wait_boot(timeout=30)
                session.type_text("run flood")
                session.key("enter")
                session.wait_text("PERF_FLOOD_DONE", timeout=15)
                elapsed = time.monotonic() - started
                rss = _rss_kb(session._proc.pid)
                report["output_flood"] = {
                    "bytes": len(payload),
                    "seconds": round(elapsed, 3),
                    "rss_kb": rss,
                }
                if elapsed > 12:
                    raise AssertionError(f"2500 字节工具输出超过 12s:{elapsed:.3f}s")
                if rss > 512 * 1024:
                    raise AssertionError(f"输出洪峰 RSS 超过 512MiB:{rss}KiB")
            finally:
                session.close()
        finally:
            sandbox.cleanup()

    result_dir = ROOT / ".test-results"
    result_dir.mkdir(exist_ok=True)
    (result_dir / "tui-performance.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print("PASS", json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
