#!/usr/bin/env python3
"""Check local documentation links/fragments and the 24-case acceptance mapping.

Read-only and offline. External URLs are references, not live verification.
"""
from pathlib import Path
import re
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]


def slugs(path):
    counts = {}
    result = set()
    for heading in re.findall(r"^#{1,6}\s+(.+)$", path.read_text(), re.M):
        slug = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        count = counts.get(slug, 0)
        counts[slug] = count + 1
        result.add(slug + (f"-{count}" if count else ""))
    return result


def check():
    files = [ROOT / "README.md", *sorted((ROOT / "docs").glob("*.md"))]
    checked = 0
    for path in files:
        for raw in re.findall(r"\[[^\]]*\]\(([^)]+)\)", path.read_text()):
            if raw.startswith(("http:", "https:", "mailto:")):
                continue
            target, _, anchor = unquote(raw.strip("<>")).partition("#")
            target = path.parent / target if target else path
            if not target.exists():
                raise ValueError(f"{path.name}: missing target {raw}")
            if anchor and target.suffix == ".md" and anchor not in slugs(target):
                raise ValueError(f"{path.name}: missing fragment {raw}")
            checked += 1
    report = (ROOT / "docs/executable-foundation.md").read_text()
    cases = re.findall(
        r"\| \[([^]]+)\]\(acceptance-cases.md#([^)]*)\) \| `([^`]+)` \| Pass \|",
        report,
    )
    if len(cases) != 24 or len({c[2] for c in cases}) != 24:
        raise ValueError("expected 24 distinct acceptance mappings")
    rust = (ROOT / "crates/koine-sqlite/tests/acceptance.rs").read_text()
    anchors = slugs(ROOT / "docs/acceptance-cases.md")
    for case, anchor, test in cases:
        if anchor not in anchors or not re.search(r"#\[test\]\s*fn " + re.escape(test) + r"\(", rust):
            raise ValueError(f"invalid acceptance mapping: {case} / {test}")
    print(f"Validated {len(files)} Markdown files, {checked} local targets/fragments, 24 acceptance mappings")


if __name__ == "__main__":
    check()
