"""Writes THIRD-PARTY.md: every Rust crate and npm package that ships in Prism Survey, with its licence, and fails when
one is not under a permissive licence (the project's rule: MIT, BSD, Apache 2.0, ISC, Zlib and the like).

    python tools/third_party.py            write THIRD-PARTY.md
    python tools/third_party.py --check    only check, and say what is not permissive

Rust crates come from `cargo tree` for Windows, normal and build dependencies (test-only crates are left out), for the
main workspace and the desktop app's workspace. npm packages come from `npm ls --omit=dev` in app/ui, with the licence
each package's own package.json gives.
"""

import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGET = "x86_64-pc-windows-msvc"
# Licence names counted as permissive. An expression passes when every AND part has a permissive choice.
PERMISSIVE = {
    "MIT", "MIT-0", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "0BSD", "ISC",
    "Zlib", "Unlicense", "BSL-1.0", "CC0-1.0", "Unicode-3.0", "Unicode-DFS-2016", "BlueOak-1.0.0", "Python-2.0",
}
OWN = re.compile(r"^prismsurvey")
# Accepted by name (Jason, 2026-11-17; docs/DECISIONS.md 55): MPL-2.0 crates Tauri needs for the window, used unchanged.
EXCEPTIONS = {("cssparser", "MPL-2.0"), ("cssparser-macros", "MPL-2.0"), ("dtoa-short", "MPL-2.0"), ("selectors", "MPL-2.0"), ("option-ext", "MPL-2.0")}


def permissive(expr):
    if not expr:
        return False
    e = expr.replace("/", " OR ").replace("(", " ").replace(")", " ")
    for part in re.split(r"\s+AND\s+", e):
        choices = [c.strip() for c in re.split(r"\s+OR\s+", part) if c.strip()]
        if not any(c in PERMISSIVE for c in choices):
            return False
    return True


def crates(manifest):
    out = subprocess.run(
        ["cargo", "tree", "--manifest-path", manifest, "--target", TARGET, "-e", "normal,build",
         "--prefix", "none", "--format", "{p}\t{l}", "--workspace"],
        capture_output=True, text=True, encoding="utf-8", check=True, cwd=ROOT,
    ).stdout
    found = {}
    for line in out.splitlines():
        line = line.replace(" (*)", "").strip()
        if "\t" not in line:
            continue
        p, lic = line.split("\t", 1)
        bits = p.split()
        if len(bits) < 2 or OWN.match(bits[0]):
            continue
        found[(bits[0], bits[1].lstrip("v"))] = lic.strip()
    return found


def npm_packages():
    ui = os.path.join(ROOT, "app", "ui")
    out = subprocess.run("npm ls --omit=dev --all --json", shell=True, capture_output=True, text=True, encoding="utf-8", cwd=ui).stdout
    tree = json.loads(out or "{}")
    found = {}

    def walk(deps):
        for name, info in (deps or {}).items():
            ver = info.get("version", "")
            pj = os.path.join(ui, "node_modules", *name.split("/"), "package.json")
            lic = ""
            if os.path.exists(pj):
                lic = json.load(open(pj, encoding="utf-8")).get("license", "")
                if isinstance(lic, dict):
                    lic = lic.get("type", "")
            found[(name, ver)] = lic
            walk(info.get("dependencies"))

    walk(tree.get("dependencies"))
    return found


def main():
    check = "--check" in sys.argv
    engine = crates(os.path.join(ROOT, "Cargo.toml"))
    app = crates(os.path.join(ROOT, "app", "src-tauri", "Cargo.toml"))
    window_only = {k: v for k, v in app.items() if k not in engine}
    npm = npm_packages()
    bad = [(k, v) for d in (engine, window_only, npm) for k, v in d.items() if not permissive(v) and (k[0], v) not in EXCEPTIONS]
    for (n, ver), lic in sorted(bad):
        print(f"not permissive or not stated: {n} {ver}: {lic or '(none)'}")
    print(f"{len(engine)} engine crates, {len(window_only)} more for the window, {len(npm)} npm packages; {len(bad)} to look at")
    if check:
        sys.exit(1 if bad else 0)

    def table(d, head):
        rows = [f"| {head} | Version | License |", "|---|---|---|"]
        rows += [f"| {n} | {v} | {lic} |" for (n, v), lic in sorted(d.items(), key=lambda x: (x[0][0].lower(), x[0][1]))]
        return rows

    lines = [
        "# Third-party components",
        "",
        "Everything built into Prism Survey that others wrote, each with its license as its authors publish it. Where a component offers a choice, Prism Survey uses it under the MIT or Apache 2.0 terms. Test-only and build-tool components that do not ship are not listed. This file is written by `python tools/third_party.py`, which also checks that every license is permissive.",
        "",
        "One exception to the permissive rule, by name: five crates the window needs (cssparser, cssparser-macros, dtoa-short, selectors and option-ext) are under MPL-2.0, which asks only that changes to those files be shared. Prism Survey uses them unchanged; their source is at crates.io.",
        "",
        "Text recognition uses the recogniser built into Windows (Windows.Media.Ocr), called through the `windows` crate; nothing is bundled for it. The window is drawn by the web view built into Windows; the installer fetches it only if the computer lacks it.",
        "",
        "## Rust crates in the engine and command-line tool",
        "",
        *table(engine, "Crate"),
        "",
        "## Rust crates the desktop window adds",
        "",
        *table(window_only, "Crate"),
        "",
        "## npm packages in the interface",
        "",
        *table(npm, "Package"),
        "",
    ]
    open(os.path.join(ROOT, "THIRD-PARTY.md"), "w", encoding="utf-8", newline="\n").write("\n".join(lines))
    print("wrote THIRD-PARTY.md")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
