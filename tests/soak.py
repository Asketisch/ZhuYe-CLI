#!/usr/bin/env python3
"""TUI 长稳冒烟:持续 resize / redraw 并观察进程存活与 RSS 趋势。"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import time
from pathlib import Path

from performance import _rss_kb
from tui_pty import Sandbox, TuiSession


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests/functional"))


def main() -> int:
    binary = Path(os.environ.get("ZHUYE_CLI_BIN", ROOT / "target/release/zhuye-cli"))
    duration = float(os.environ.get("TUI_SOAK_SECONDS", "30"))
    if not binary.is_file():
        print("[ERROR] 缺少 release 二进制;先 cargo build --release -p zhuye-cli", file=sys.stderr)
        return 2
    if duration < 5:
        print("[ERROR] TUI_SOAK_SECONDS 不能低于 5 秒", file=sys.stderr)
        return 2

    with tempfile.TemporaryDirectory(prefix="zhuye-soak-") as temp_name:
        sandbox = Sandbox.create(str(Path(temp_name) / "root"))
        session = TuiSession(
            sandbox,
            env=sandbox.mock_env(),
            cols=110,
            rows=34,
        )
        try:
            session.wait_boot(timeout=30)
            samples: list[int] = []
            started = time.monotonic()
            sizes = ((100, 30), (110, 34), (132, 40), (80, 24))
            index = 0
            while time.monotonic() - started < duration:
                cols, rows = sizes[index % len(sizes)]
                session.resize(cols, rows)
                index += 1
                time.sleep(1)
                samples.append(_rss_kb(session._proc.pid))
            if not session.alive():
                raise AssertionError("长稳期间进程退出")
            if session.has_panic():
                raise AssertionError("长稳期间出现 panic")
            growth = max(samples) - min(samples)
            if growth > 64 * 1024:
                raise AssertionError(f"长稳 RSS 波动超过 64MiB:{growth}KiB;samples={samples}")
        finally:
            session.close()
            sandbox.cleanup()

    report = {
        "duration_seconds": duration,
        "resize_cycles": index,
        "rss_samples_kb": samples,
        "rss_growth_kb": growth,
    }
    result_dir = ROOT / ".test-results"
    result_dir.mkdir(exist_ok=True)
    (result_dir / "tui-soak.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print("PASS", json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
