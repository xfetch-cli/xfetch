#!/usr/bin/env python3
"""Generate docs/ATTRIBUTIONS.md from the locked dependency graph.

This is a local maintenance helper: it does not run during the build and does
not change Cargo.toml or Cargo.lock.

Usage:
    python3 scripts/generate_attributions.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "docs" / "ATTRIBUTIONS.md"


def cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def runtime_packages(metadata: dict) -> tuple[set[str], set[str]]:
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root_id = metadata["resolve"]["root"]
    direct = {
        dep["pkg"]
        for dep in nodes[root_id]["deps"]
        if any(kind.get("kind") is None for kind in dep["dep_kinds"])
    }

    seen: set[str] = set()
    pending = [root_id]
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        for dep in nodes.get(package_id, {}).get("deps", []):
            if any(kind.get("kind") is None for kind in dep["dep_kinds"]):
                pending.append(dep["pkg"])

    seen.discard(root_id)
    return seen, direct


def license_of(package: dict) -> str:
    license_id = package.get("license")
    if license_id:
        return license_id
    license_file = package.get("license_file")
    if license_file:
        return f"see {Path(license_file).name}"
    return "unknown"


def link_of(package: dict) -> str:
    repository = package.get("repository")
    if repository:
        return repository
    return f"https://crates.io/crates/{package['name']}/{package['version']}"


def escape(value: str) -> str:
    return value.replace("|", "\\|")


def main() -> int:
    metadata = cargo_metadata()
    packages = {package["id"]: package for package in metadata["packages"]}
    ids, direct = runtime_packages(metadata)

    rows = []
    for package_id in ids:
        package = packages[package_id]
        rows.append(
            (
                package["name"],
                package["version"],
                license_of(package),
                "yes" if package_id in direct else "no",
                link_of(package),
            )
        )
    rows.sort(key=lambda row: (row[0].lower(), row[1]))

    lines = [
        "# Dependency attributions",
        "",
        "Runtime dependencies of the default feature set across supported",
        "platforms, generated from `Cargo.lock`. This file is informational and",
        "is not part of the build.",
        "",
        "Regenerate with:",
        "",
        "```bash",
        "python3 scripts/generate_attributions.py",
        "```",
        "",
        "| Crate | Version | License | Direct | Link |",
        "| --- | --- | --- | --- | --- |",
    ]
    for name, version, license_id, is_direct, link in rows:
        lines.append(
            f"| `{escape(name)}` | {escape(version)} | {escape(license_id)} "
            f"| {is_direct} | <{link}> |"
        )

    lgpl = [
        row
        for row in rows
        if "LGPL" in row[2].upper() and " OR " not in row[2].upper()
    ]
    if lgpl:
        lines.extend(
            [
                "",
                "## License notes",
                "",
                "The following crates are licensed only under an LGPL license.",
                "Binary distributions that link them must also comply with the",
                "LGPL, including license notices and a way to replace or relink",
                "the library:",
                "",
            ]
        )
        for name, version, license_id, _, link in lgpl:
            lines.append(f"- `{name}` {version} ({license_id}): <{link}>")

    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Wrote {OUTPUT.relative_to(ROOT)} ({len(rows)} packages).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
