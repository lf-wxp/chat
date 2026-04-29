#!/usr/bin/env python3
"""Check that en.json and zh-CN.json have identical key structures.

Compares the nested key hierarchy of two i18n JSON files and reports
any keys that are present in one file but missing from the other.
Exits with code 1 if mismatches are found, 0 otherwise.

Usage:
    python3 scripts/check_i18n_keys.py [en.json] [zh-CN.json]
    # Defaults to frontend/locales/en.json and frontend/locales/zh-CN.json
"""

import json
import sys
from pathlib import Path
from typing import Set


def collect_keys(obj: dict, prefix: str = "") -> Set[str]:
    """Recursively collect all dot-separated key paths from a nested dict."""
    keys: set[str] = set()
    for key, value in obj.items():
        full = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            keys.update(collect_keys(value, full))
        else:
            keys.add(full)
    return keys


def main() -> None:
    base = Path(__file__).resolve().parent.parent / "frontend" / "locales"
    en_path = Path(sys.argv[1]) if len(sys.argv) > 1 else base / "en.json"
    zh_path = Path(sys.argv[2]) if len(sys.argv) > 2 else base / "zh-CN.json"

    en_data = json.loads(en_path.read_text(encoding="utf-8"))
    zh_data = json.loads(zh_path.read_text(encoding="utf-8"))

    en_keys = collect_keys(en_data)
    zh_keys = collect_keys(zh_data)

    missing_in_zh = sorted(en_keys - zh_keys)
    missing_in_en = sorted(zh_keys - en_keys)

    ok = True
    if missing_in_zh:
        ok = False
        print(f"❌ Keys in en.json but missing from zh-CN.json ({len(missing_in_zh)}):")
        for k in missing_in_zh:
            print(f"   - {k}")
    if missing_in_en:
        ok = False
        print(f"❌ Keys in zh-CN.json but missing from en.json ({len(missing_in_en)}):")
        for k in missing_in_en:
            print(f"   - {k}")
    if ok:
        print(f"✅ i18n key consistency check passed ({len(en_keys)} keys)")

    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
