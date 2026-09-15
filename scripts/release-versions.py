#!/usr/bin/env python3
"""Single source of truth for the SDK's three independent version tracks.

Tracks
------
* bindings : Python (py/pyproject.toml, PEP 440) + Node.js (js/package.json,
             SemVer) + the Cargo workspace version the py/js crates inherit.
* rust     : crates.io crates fugle-marketdata-core and fugle-marketdata,
             plus the workspace dependency alias that pins core.
* uniffi   : C# / Go / Java / C++ (uniffi/Cargo.toml, the .csproj and the
             Gradle default).

Usage
-----
    scripts/release-versions.py check
        Verify every manifest agrees within its track. Exit 1 on drift.

    scripts/release-versions.py resolve --tag v3.0.0-rc.1 [--github-output FILE]
        Run `check`, then require the tag to equal the bindings version.
        Prints (and optionally appends to $GITHUB_OUTPUT) the values the
        release workflow needs.

    scripts/release-versions.py require-rc --track rust
        Fail unless the given track's version is a release candidate.

The tag names a *bindings* release. Rust crates are tagged `rust-vX.Y.Z` and
released by .github/workflows/release-rust.yml; see docs/RELEASING.md.

Release policy: only release candidates (`X.Y.Z-rc.N`) may be published, on
every registry. `resolve` enforces this for the bindings and UniFFI tracks and
`require-rc` for the Rust track. There is deliberately no flag to bypass it;
publishing a stable version requires changing RC_ONLY below in a reviewed PR.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Publishing policy: release candidates only. See the module docstring.
RC_ONLY = True
RC = re.compile(r"^\d+\.\d+\.\d+-rc\.\d+$")

SEMVER = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta|rc)\.(\d+))?$")
PEP440_PRE = {"alpha": "a", "beta": "b", "rc": "rc"}


def toml(path: str) -> dict:
    return tomllib.loads((ROOT / path).read_text())


def semver_to_pep440(v: str) -> str:
    m = SEMVER.match(v)
    if not m:
        raise ValueError(f"unsupported version {v!r} (X.Y.Z or X.Y.Z-{{alpha,beta,rc}}.N)")
    base = ".".join(m.group(i) for i in (1, 2, 3))
    return base if not m.group(4) else f"{base}{PEP440_PRE[m.group(4)]}{m.group(5)}"


def collect() -> dict[str, dict[str, str]]:
    workspace = toml("Cargo.toml")
    csproj = (ROOT / "bindings/csharp/MarketdataUniffi/MarketdataUniffi.csproj").read_text()
    gradle = (ROOT / "bindings/java/build.gradle.kts").read_text()
    cs = re.search(r"<Version>([^<]+)</Version>", csproj)
    java = re.search(r'gradleProperty\("projectVersion"\)\.getOrElse\("([^"]+)"\)', gradle)
    return {
        "bindings": {
            "js/package.json": json.loads((ROOT / "js/package.json").read_text())["version"],
            "Cargo.toml [workspace.package]": workspace["workspace"]["package"]["version"],
            "py/pyproject.toml (PEP 440)": toml("py/pyproject.toml")["project"]["version"],
        },
        "rust": {
            "core/Cargo.toml": toml("core/Cargo.toml")["package"]["version"],
            "rust/Cargo.toml": toml("rust/Cargo.toml")["package"]["version"],
            "Cargo.toml marketdata-core alias": workspace["workspace"]["dependencies"]["marketdata-core"]["version"],
        },
        "uniffi": {
            "uniffi/Cargo.toml": toml("uniffi/Cargo.toml")["package"]["version"],
            "MarketdataUniffi.csproj <Version>": cs.group(1) if cs else "<missing>",
            "build.gradle.kts projectVersion default": java.group(1) if java else "<missing>",
        },
    }


def check(versions: dict[str, dict[str, str]]) -> list[str]:
    errors = []
    for track, sources in versions.items():
        items = list(sources.items())
        ref_name, ref = items[0]
        if not SEMVER.match(ref):
            errors.append(f"[{track}] {ref_name} = {ref!r} is not a supported SemVer version")
            continue
        for name, value in items[1:]:
            expected = semver_to_pep440(ref) if "PEP 440" in name else ref
            if value != expected:
                errors.append(f"[{track}] {name} = {value!r}, expected {expected!r} (from {ref_name})")
    return errors


def rc_errors(versions: dict[str, dict[str, str]], tracks: list[str]) -> list[str]:
    if not RC_ONLY:
        return []
    ref = {"bindings": "js/package.json", "rust": "core/Cargo.toml", "uniffi": "uniffi/Cargo.toml"}
    return [
        f"[{t}] {ref[t]} = {versions[t][ref[t]]!r} is not a release candidate (X.Y.Z-rc.N). "
        "Only release candidates may be published."
        for t in tracks
        if not RC.match(versions[t][ref[t]])
    ]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    r = sub.add_parser("resolve")
    r.add_argument("--tag", required=True)
    r.add_argument("--github-output")
    rc = sub.add_parser("require-rc")
    rc.add_argument("--track", required=True, choices=["bindings", "rust", "uniffi"], action="append")
    args = ap.parse_args()

    versions = collect()
    for track, sources in versions.items():
        for name, value in sources.items():
            print(f"{track:<9} {name:<42} {value}")
    errors = check(versions)
    for e in errors:
        print(f"::error::{e}")
    if errors:
        return 1
    if args.cmd == "check":
        print("All version tracks are internally consistent.")
        return 0
    if args.cmd == "require-rc":
        rc_errs = rc_errors(versions, args.track)
        for e in rc_errs:
            print(f"::error::{e}")
        return 1 if rc_errs else 0

    rc_errs = rc_errors(versions, ["bindings", "uniffi"])
    for e in rc_errs:
        print(f"::error::{e}")
    if rc_errs:
        return 1

    bindings = versions["bindings"]["js/package.json"]
    tag_version = args.tag.removeprefix("refs/tags/").removeprefix("v")
    if tag_version != bindings:
        print(
            f"::error::Tag {args.tag} does not match the bindings version {bindings}. "
            "Release tags name the bindings track (py/js); Rust crates use rust-vX.Y.Z tags."
        )
        return 1
    uniffi = versions["uniffi"]["uniffi/Cargo.toml"]
    prerelease = "-" in bindings
    out = {
        "version": bindings,
        "python_version": semver_to_pep440(bindings),
        "uniffi_version": uniffi,
        "rust_version": versions["rust"]["core/Cargo.toml"],
        "channel": "prerelease" if prerelease else "stable",
        "npm_tag": "next" if prerelease else "latest",
    }
    for k, v in out.items():
        print(f"{k}={v}")
    if args.github_output:
        with open(args.github_output, "a") as fh:
            fh.writelines(f"{k}={v}\n" for k, v in out.items())
    return 0


if __name__ == "__main__":
    sys.exit(main())
