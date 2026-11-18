#!/usr/bin/env python3
"""检查差分报告是否真正执行了所有预期场景。"""

import sys
import xml.etree.ElementTree as ET
from pathlib import Path

report = Path(sys.argv[1])
expected = set(sys.argv[2:])
root = ET.parse(report).getroot()
cases = {case.get("name", ""): case for case in root.iter("testcase")}
for name in sorted(expected):
    case = cases.get(name)
    if case is None:
        raise SystemExit(f"差分报告缺少场景:{name}")
    if any(case.find(status) is not None for status in ("skipped", "failure", "error")):
        raise SystemExit(f"差分场景没有通过:{name}")
print(f"差分报告已验证 {len(expected)} 个场景均已执行并通过")
