#!/usr/bin/env python3
"""Changelog fragments: one file per change in changes/, assembled into
CHANGELOG.md when a release is cut, so two pull requests never edit the same
lines.

    python3 scripts/changelog.py check                   validate the fragments
    python3 scripts/changelog.py preview                 print the next release's notes
    python3 scripts/changelog.py release <version> [date]
                                                         write the section into CHANGELOG.md
                                                         and delete the fragments

A fragment is changes/<slug>.<kind>.md holding the entry's text (no leading
"- "; continuation lines are fine). Kinds, in the order they appear:
added, changed, output, deprecated, removed, fixed, performance, security.
`output` is the "Output changes" heading (see the top of CHANGELOG.md).
Standard library only.
"""
from __future__ import annotations

import re
import sys
from datetime import date as _date
from pathlib import Path

KINDS = [
    ("added", "Added"),
    ("changed", "Changed"),
    ("output", "Output changes"),
    ("deprecated", "Deprecated"),
    ("removed", "Removed"),
    ("fixed", "Fixed"),
    ("performance", "Performance"),
    ("security", "Security"),
]
UNRELEASED = "## [Unreleased]"
POINTER = "Pending changes are files in [`changes/`](./changes/README.md)."


def fragments(directory: Path) -> list[dict]:
    if not directory.is_dir():
        return []
    out = []
    for path in sorted(directory.glob("*.md")):
        if path.name == "README.md":
            continue
        parts = path.name[:-3].split(".")
        out.append(
            {
                "name": path.name,
                "slug": ".".join(parts[:-1]),
                "kind": parts[-1],
                "text": path.read_text(encoding="utf-8").strip(),
            }
        )
    return out


def problems(items: list[dict]) -> list[str]:
    known = [kind for kind, _ in KINDS]
    out = []
    for f in items:
        if not f["slug"] or f["kind"] not in known:
            out.append(f"{f['name']}: name it <slug>.<kind>.md with kind one of {', '.join(known)}")
        if not f["text"]:
            out.append(f"{f['name']}: empty")
        if re.match(r"[-*] ", f["text"]):
            out.append(f"{f['name']}: write the text without the leading \"- \"")
        if re.search(r"^#{1,3} ", f["text"], re.M):
            out.append(f"{f['name']}: no headings inside an entry")
    return out


def bullet(text: str) -> str:
    first, *rest = text.split("\n")
    return "\n".join([f"- {first}", *[f"  {line}" if line else "" for line in rest]])


def body(items: list[dict]) -> str:
    sections = []
    for kind, heading in KINDS:
        entries = [f for f in items if f["kind"] == kind]
        if entries:
            sections.append(f"### {heading}\n\n" + "\n".join(bullet(f["text"]) for f in entries))
    return "\n\n".join(sections)


def release(changelog: str, version: str, day: str, items: list[dict]) -> str:
    lines = changelog.split("\n")
    for line in lines:
        if line.startswith(f"## [{version}]"):
            raise ValueError(f"CHANGELOG.md already has a section for {version}")
    at = next((i for i, line in enumerate(lines) if line.startswith(UNRELEASED)), None)
    if at is None:
        raise ValueError(f"CHANGELOG.md has no {UNRELEASED} heading")
    end = at + 1
    while end < len(lines) and not lines[end].startswith("## "):
        end += 1
    section = f"## [{version}] — {day}\n\n{body(items)}\n"
    new = [*lines[: at + 1], "", POINTER, "", *section.split("\n"), *lines[end:]]
    return "\n".join(new)


def main(argv: list[str], root: Path) -> int:
    command = argv[0] if argv else ""
    directory = root / "changes"
    items = fragments(directory)
    bad = problems(items)
    if bad:
        print("\n".join(bad), file=sys.stderr)
        return 1
    if command == "check":
        print(f"{len(items)} changelog fragment(s) ok")
        return 0
    if command == "preview":
        if not items:
            print("changes/ has no fragments", file=sys.stderr)
            return 1
        print(body(items))
        return 0
    if command == "release":
        version = argv[1] if len(argv) > 1 else ""
        if not re.fullmatch(r"\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?", version):
            print("usage: changelog.py release <version, e.g. 0.7.0> [YYYY-MM-DD]", file=sys.stderr)
            return 2
        if not items:
            print("changes/ has no fragments to release", file=sys.stderr)
            return 1
        day = argv[2] if len(argv) > 2 else _date.today().isoformat()
        path = root / "CHANGELOG.md"
        path.write_text(release(path.read_text(encoding="utf-8"), version, day, items), encoding="utf-8")
        for f in items:
            (directory / f["name"]).unlink()
        print(f"CHANGELOG.md: ## [{version}] — {day} from {len(items)} fragment(s)")
        return 0
    print("usage: changelog.py check | preview | release <version> [date]", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:], Path(__file__).resolve().parent.parent))
