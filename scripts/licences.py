#!/usr/bin/env python3
"""Collect what the app ships and under what licence, into src/lib/licences.json.

Four ecosystems end up in one build and every one of them answers differently:

  rust     `cargo tree -e normal --target aarch64-linux-android` — what is linked
           into the core, not what the build used on the way there.
  web      the production npm tree, which is what the webview bundle carries.
  android  the Gradle lines of the app and of its three plugins, read from the
           build files rather than typed here, so the list cannot drift.
  ios      the Swift packages the iOS plugins pull in.

Licence texts are shared: one copy per SPDX id, taken from a crate that ships it,
with each library's own copyright line kept beside the library. That is what a
store listing and an About screen need, and it stays under thirty kilobytes.

    python3 scripts/licences.py            # writes src/lib/licences.json
    python3 scripts/licences.py --check    # fails when the file is out of date
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src" / "lib" / "licences.json"
CARGO_SRC = Path.home() / ".cargo" / "registry" / "src"

# Which licence each text belongs to, by the file name crates use for it.
TEXT_FILES = {
    "LICENSE-MIT": "MIT",
    "LICENSE-APACHE": "Apache-2.0",
    "LICENSE-BSD-3-Clause": "BSD-3-Clause",
    "LICENSE-UNICODE": "Unicode-3.0",
    "LICENSE-ZLIB": "Zlib",
    "LICENSE-MPL-2.0": "MPL-2.0",
    "UNLICENSE": "Unlicense",
    "LICENSE-0BSD": "0BSD",
}

# A real notice starts the line and names a year; the Apache text is full of
# prose about copyright notices, and its appendix says "[yyyy]".
COPYRIGHT = re.compile(r"^(Copyright\s+(?:\(c\)|©)?\s*\d{4}[^\n]*)$", re.MULTILINE)


def run(cmd: list[str], cwd: Path = ROOT) -> str:
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, check=False).stdout


# ---- rust -------------------------------------------------------------------

OURS = {"encedo-authenticator", "encedo-protocol", "tauri-plugin-encedo-push", "tauri-plugin-encedo-keystore", "tauri-plugin-barcode-scanner"}


def crate_dir(name: str, version: str) -> Path | None:
    for registry in CARGO_SRC.glob("*"):
        d = registry / f"{name}-{version}"
        if d.is_dir():
            return d
    return None


def copyright_of(directory: Path | None) -> str:
    """The first copyright line of whatever licence file the project ships."""
    if not directory:
        return ""
    for pattern in ("LICENSE*", "LICENCE*", "COPYING*", "NOTICE*"):
        for path in sorted(directory.glob(pattern)):
            if path.is_dir():
                continue
            try:
                m = COPYRIGHT.search(path.read_text(errors="ignore")[:4000])
            except OSError:
                continue
            if m:
                return " ".join(m.group(1).split())[:120]
    return ""


def rust_items(texts: dict[str, str]) -> list[dict]:
    raw = run(["cargo", "tree", "-e", "normal", "--target", "aarch64-linux-android", "--prefix", "none", "--format", "{p}|{l}"], ROOT / "src-tauri")
    seen: dict[tuple[str, str], str] = {}
    for line in raw.splitlines():
        if "|" not in line:
            continue
        left, licence = line.split("|", 1)
        parts = left.split()
        if len(parts) < 2 or not parts[1].startswith("v"):
            continue
        name, version = parts[0], parts[1][1:]
        if name in OURS:
            continue
        # cargo tree marks a repeated subtree with "(*)", which lands after the
        # licence in this format.
        seen[(name, version)] = licence.strip().removesuffix("(*)").strip() or "?"
    items = []
    for (name, version), licence in sorted(seen.items()):
        directory = crate_dir(name, version)
        collect_texts(directory, texts)
        item = {"n": name, "v": version, "l": licence}
        holder = copyright_of(directory)
        if holder:
            item["c"] = holder
        items.append(item)
    return items


def collect_texts(directory: Path | None, texts: dict[str, str]) -> None:
    if not directory:
        return
    for path in directory.glob("LICENSE*"):
        spdx = TEXT_FILES.get(path.name)
        if spdx and spdx not in texts and path.is_file():
            body = path.read_text(errors="ignore").strip()
            # The copyright line belongs to that one crate, not to the shared text.
            texts[spdx] = COPYRIGHT.sub("Copyright (c) <the holders named beside each library above>", body, count=1)


# ---- web --------------------------------------------------------------------


def web_items() -> list[dict]:
    raw = run(["npm", "ls", "--prod", "--all", "--json"])
    try:
        tree = json.loads(raw)
    except json.JSONDecodeError:
        return []
    found: dict[tuple[str, str], dict] = {}

    def walk(node: dict) -> None:
        for name, dep in (node.get("dependencies") or {}).items():
            version = dep.get("version", "")
            pkg = ROOT / "node_modules" / name / "package.json"
            licence = "?"
            if pkg.is_file():
                try:
                    meta = json.loads(pkg.read_text())
                    licence = meta.get("license") or "?"
                except json.JSONDecodeError:
                    pass
            item = {"n": name, "v": version, "l": licence}
            holder = copyright_of(ROOT / "node_modules" / name)
            if holder:
                item["c"] = holder
            found[(name, version)] = item
            walk(dep)

    walk(tree)
    # Compiled into the bundle rather than installed beside it, so `--prod` does
    # not see them, but their code ships all the same.
    for name in ("svelte", "esm-env"):
        pkg = ROOT / "node_modules" / name / "package.json"
        if not pkg.is_file():
            continue
        meta = json.loads(pkg.read_text())
        item = {"n": name, "v": meta.get("version", ""), "l": meta.get("license") or "?"}
        holder = copyright_of(ROOT / "node_modules" / name)
        if holder:
            item["c"] = holder
        found[(name, item["v"])] = item
    return [found[k] for k in sorted(found)]


# ---- native -----------------------------------------------------------------

GRADLE_DEP = re.compile(r'implementation\("([^":]+):([^":]+):([^"]+)"\)')

# Gradle coordinates carry no licence, and these four publishers state theirs in
# their POMs rather than in the tree. Every Google and JetBrains artifact below
# ships under Apache-2.0; anything new shows up as "?" and has to be looked up.
NATIVE_LICENCES = {
    "androidx": "Apache-2.0",
    "com.google.android.material": "Apache-2.0",
    "com.google.android.gms": "Apache-2.0",
    "com.google.firebase": "Apache-2.0",
    "org.jetbrains.kotlin": "Apache-2.0",
    "org.jetbrains.kotlinx": "Apache-2.0",
}


def android_items() -> list[dict]:
    files = [
        ROOT / "src-tauri" / "gen" / "android" / "app" / "build.gradle.kts",
        ROOT / "src-tauri" / "plugins" / "barcode-scanner" / "android" / "build.gradle.kts",
        ROOT / "src-tauri" / "plugins" / "encedo-push" / "android" / "build.gradle.kts",
        ROOT / "src-tauri" / "plugins" / "encedo-keystore" / "android" / "build.gradle.kts",
    ]
    versions: dict[str, str] = {}
    found: dict[str, dict] = {}
    for path in files:
        if not path.is_file():
            continue
        text = path.read_text()
        for name, value in re.findall(r'val\s+(\w+)\s*=\s*"([^"]+)"', text):
            versions[name] = value
        for group, artifact, version in GRADLE_DEP.findall(text):
            if version.startswith("$"):
                version = versions.get(version.lstrip("${").rstrip("}"), version)
            family = ".".join(group.split(".")[:2]) if group.startswith("androidx") else group
            licence = NATIVE_LICENCES.get("androidx" if group.startswith("androidx") else group, "?")
            found[f"{group}:{artifact}"] = {"n": f"{group}:{artifact}", "v": version, "l": licence}
            _ = family
    # The toolchain itself, which no build file names but every build carries.
    found["org.jetbrains.kotlin:kotlin-stdlib"] = {"n": "org.jetbrains.kotlin:kotlin-stdlib", "v": "bundled", "l": "Apache-2.0"}
    found["tauri-android"] = {"n": "app.tauri:tauri-android", "v": "2", "l": "Apache-2.0 OR MIT", "c": "Copyright (c) 2019 - Present Tauri Programme within The Commons Conservancy"}
    return [found[k] for k in sorted(found)]


SPM_DEP = re.compile(r'\.package\(\s*url:\s*"([^"]+)"[^)]*?(?:from|exact|branch):\s*"([^"]+)"', re.DOTALL)


def ios_items() -> list[dict]:
    found: dict[str, dict] = {}
    for path in ROOT.glob("src-tauri/plugins/*/ios/Package.swift"):
        for url, version in SPM_DEP.findall(path.read_text()):
            name = url.rstrip("/").split("/")[-1].removesuffix(".git")
            licence = "Apache-2.0" if "firebase" in name.lower() else "?"
            found[name] = {"n": name, "v": version, "l": licence, "u": url}
    found["Tauri"] = {"n": "tauri-apps/tauri (iOS runtime)", "v": "2", "l": "Apache-2.0 OR MIT"}
    return [found[k] for k in sorted(found)]


# ---- writing ----------------------------------------------------------------


# Where a licence has no two-file convention, take the text from a crate that
# ships that licence alone.
PLAIN_TEXT_FILES = ("LICENSE", "LICENSE.txt", "LICENSE.md", "LICENCE", "COPYING")


def fill_missing_texts(items: list[dict], texts: dict[str, str]) -> None:
    """Every licence named above gets its text, from a library that ships it."""
    wanted: set[str] = set()
    for item in items:
        for part in re.split(r"\s+(?:OR|AND)\s+|/", item["l"]):
            part = part.strip()
            if part and part != "?":
                wanted.add(part)
    for spdx in sorted(wanted - set(texts)):
        for item in items:
            if item["l"].strip() != spdx:
                continue
            directory = crate_dir(item["n"], item["v"])
            if not directory:
                continue
            for name in PLAIN_TEXT_FILES:
                path = directory / name
                if path.is_file():
                    body = path.read_text(errors="ignore").strip()
                    texts[spdx] = COPYRIGHT.sub("Copyright (c) <the holders named beside each library above>", body, count=1)
                    break
            if spdx in texts:
                break


def build() -> dict:
    texts: dict[str, str] = {}
    rust = rust_items(texts)
    fill_missing_texts(rust, texts)
    data = {
        "generated": date.today().isoformat(),
        "groups": [
            {"id": "rust", "short": "Rust", "name": "Rust, in the app's core", "items": rust},
            {"id": "web", "short": "JavaScript", "name": "JavaScript, in the screens", "items": web_items()},
            {"id": "android", "short": "Android", "name": "Android libraries", "items": android_items()},
            {"id": "ios", "short": "iOS", "name": "iOS libraries", "items": ios_items()},
        ],
        "texts": dict(sorted(texts.items())),
    }
    return data


def main() -> int:
    data = build()
    rendered = json.dumps(data, indent=1, ensure_ascii=False, sort_keys=False) + "\n"
    total = sum(len(g["items"]) for g in data["groups"])
    unknown = [i["n"] for g in data["groups"] for i in g["items"] if i["l"] == "?"]
    if "--check" in sys.argv:
        current = OUT.read_text() if OUT.is_file() else ""
        # The date alone must not fail the check.
        same = re.sub(r'"generated": "[^"]+"', "", current) == re.sub(r'"generated": "[^"]+"', "", rendered)
        print(f"{'up to date' if same else 'OUT OF DATE'}: {total} libraries, {len(data['texts'])} licence texts")
        return 0 if same else 1
    OUT.write_text(rendered)
    print(f"{OUT.relative_to(ROOT)}: {total} libraries, {len(data['texts'])} licence texts, {len(rendered) / 1024:.1f} KB")
    if unknown:
        print(f"licence not known for {len(unknown)}: {', '.join(unknown[:8])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
