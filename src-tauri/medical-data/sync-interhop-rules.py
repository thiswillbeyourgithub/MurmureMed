#!/usr/bin/env python
"""Refresh the bundled InterHop dictation rules from a local clone of
https://framagit.org/interhop/murmure-regex (written with Claude Code).

    python sync-interhop-rules.py [path/to/murmure-regex]

Reads the clone's regex.csv with the clone's own converter
(convert_csv_to_murmure.py: same parsing, same skipped rows, same ids as its
config.murmure), and writes interhop/rules.json: one object per rule with its
id, trigger, replacement and example. Plumed seeds these as the default
formatting rules of a fresh install; the Rust tests check every example.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_REPO = HERE / "../../../parakeet_web/framagit_murmure"
CSV_NAME = "regex.csv"


def load_converter(repo: Path):
    spec = importlib.util.spec_from_file_location(
        "convert_csv_to_murmure", repo / "convert_csv_to_murmure.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    ).stdout.strip()


def main() -> None:
    repo = Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_REPO).resolve()
    csv_path = repo / CSV_NAME
    if not csv_path.is_file():
        raise SystemExit(f"ERROR: {csv_path} not found")
    converter = load_converter(repo)

    rules = []
    for row in converter.parse_csv(csv_path):
        if converter.has_lookaround(row["trigger"]):
            print(f"skipped (lookaround): {row['id']}", file=sys.stderr)
            continue
        rules.append({
            "id": converter.deterministic_rule_id(
                uuid.NAMESPACE_URL, CSV_NAME, row["example_pre"], row["example_post"]
            ),
            "trigger": row["trigger"],
            "replacement": row["replacement"],
            "example_pre": row["example_pre"],
            "example_post": row["example_post"],
        })

    out = HERE / "interhop"
    out.mkdir(exist_ok=True)
    (out / "rules.json").write_text(
        json.dumps(rules, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    commit = git(repo, "log", "-1", "--format=%H %cs", "--", CSV_NAME)
    if git(repo, "status", "--porcelain", "--", CSV_NAME):
        commit += " (plus uncommitted changes)"
    (out / "PROVENANCE").write_text(f"{CSV_NAME} {commit}\n", encoding="utf-8")
    print(f"{len(rules)} rules from {CSV_NAME} {commit}")
    print("Now run: cd src-tauri && cargo test --lib interhop")


if __name__ == "__main__":
    main()
