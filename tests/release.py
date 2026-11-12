#!/usr/bin/env python3
"""TUI 发布产物审查:压缩包结构、版本一致性与本机可执行冒烟。"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def _fail(message: str) -> None:
    print(f"[ERROR] {message}", file=sys.stderr)
    raise SystemExit(1)


def _version() -> str:
    output = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return str(json.loads(output)["packages"][0]["version"])


def _archive_names(artifact: Path) -> list[str]:
    if artifact.suffix == ".gz":
        with tarfile.open(artifact, "r:gz") as archive:
            return archive.getnames()
    with zipfile.ZipFile(artifact) as archive:
        return archive.namelist()


def _extract(artifact: Path, destination: Path) -> None:
    if artifact.suffix == ".gz":
        with tarfile.open(artifact, "r:gz") as archive:
            archive.extractall(destination, filter="data")
    else:
        with zipfile.ZipFile(artifact) as archive:
            archive.extractall(destination)


def _host_archive(name: str) -> bool:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin":
        return "-apple-darwin" in name and (machine in name or "arm64" in name)
    if system == "linux":
        return "-unknown-linux" in name
    return "-pc-windows" in name


def main() -> int:
    artifact = Path(os.environ.get("RUNNER_ARTIFACT") or (sys.argv[1] if len(sys.argv) > 1 else ""))
    if not artifact.is_file():
        print("[ERROR] 缺少发布产物;先 ./scripts/build.sh --dist", file=sys.stderr)
        return 2
    if artifact.suffix not in (".gz", ".zip"):
        _fail(f"不支持的发布产物格式:{artifact.name}")

    size = artifact.stat().st_size
    if size < 1024 * 1024:
        _fail(f"发布产物小于 1MiB:{size} bytes")
    if size > 128 * 1024 * 1024:
        _fail(f"发布产物大于 128MiB:{size} bytes")

    version = _version()
    if f"-{version}-" not in artifact.name:
        _fail(f"发布产物未包含当前版本 {version}:{artifact.name}")
    names = _archive_names(artifact)
    binary_names = [
        name
        for name in names
        if name.rstrip("/").endswith("/zhuye-cli")
        or name.rstrip("/").endswith("/zhuye-cli.exe")
        or name.rstrip("/") in ("zhuye-cli", "zhuye-cli.exe")
    ]
    if len(binary_names) != 1:
        _fail(f"发布产物应恰好包含一个二进制,实际 {len(binary_names)}:{binary_names}")
    if not any(name.endswith("/README.txt") for name in names):
        _fail("发布产物缺少 README.txt")
    if any("/target/" in name or name.endswith(".dSYM") for name in names):
        _fail("发布产物包含 target 或 dSYM 调试残留")

    report: dict[str, object] = {
        "artifact": str(artifact),
        "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
        "size_bytes": size,
        "version": version,
        "entries": names,
        "executable_smoke": "not-host-archive",
    }

    with tempfile.TemporaryDirectory(prefix="zhuye-release-check-") as temp_name:
        destination = Path(temp_name)
        _extract(artifact, destination)
        binary = destination / binary_names[0]
        if _host_archive(artifact.name):
            if platform.system() != "Windows":
                binary.chmod(0o755)
            result = subprocess.run(
                [str(binary), "--version"],
                capture_output=True,
                text=True,
                timeout=15,
            )
            report["executable_smoke"] = {
                "returncode": result.returncode,
                "stdout": result.stdout.strip(),
                "stderr": result.stderr.strip(),
            }
            if result.returncode != 0 or version not in result.stdout + result.stderr:
                _fail(f"发布二进制 --version 失败或版本不匹配:{result.stdout}{result.stderr}")

            # Windows 分发是 zip 手动解包,install.sh 只覆盖 Unix 安装矩阵。
            if platform.system() != "Windows":
                install_dir = destination / "bin"
                installed = install_dir / binary.name
                install_dir.mkdir()
                installed.write_text("stale-installation\n")
                result = subprocess.run(
                    [
                        str(ROOT / "scripts/install.sh"),
                        "--install-dir",
                        str(install_dir),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=120,
                    env={
                        "PATH": f"{install_dir}{os.pathsep}{os.environ.get('PATH', '')}",
                        "HOME": str(destination / "home"),
                    },
                )
                report["install_upgrade_uninstall"] = {
                    "returncode": result.returncode,
                    "stdout_tail": result.stdout[-1000:],
                    "stderr_tail": result.stderr[-1000:],
                }
                if result.returncode != 0:
                    _fail(f"安装脚本失败:{result.stdout}{result.stderr}")
                if not installed.is_file() or installed.stat().st_size <= len(b"stale-installation\n"):
                    _fail("安装脚本没有用真实二进制覆盖旧安装")
                installed.unlink()
                if installed.exists():
                    _fail("卸载后二进制仍存在")

    result_dir = ROOT / ".test-results"
    result_dir.mkdir(exist_ok=True)
    (result_dir / "tui-release.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print("PASS", artifact.name, report["sha256"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
