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
        Verify every manifest agrees within its track, then verify the
        *derived* locations (generated files and hand-written docs that embed
        a version) agree with the manifests. Exit 1 on drift; every error
        names the command or file that fixes it.

    scripts/release-versions.py bump [--bindings V] [--rust V] [--uniffi V] [--dry-run]
        Rewrite the manifests (and the two docs with a fixed version slot) for
        the given tracks. V is an explicit version (3.0.0-rc.5) or the literal
        `rc`, meaning "current rc number + 1". Tracks not given are untouched.
        Generated files (js/index.js, Cargo.lock, js/package-lock.json) are
        never edited; the command ends with the list of commands that
        regenerate them. Afterwards re-reads the manifests and docs the way
        `check` does and restores every file if anything disagrees, so a
        failed bump leaves no half-applied state. (A full `check` is expected
        to fail until the generated files are regenerated.)

    scripts/release-versions.py resolve --tag v3.0.0-rc.1 [--github-output FILE]
        Run `check`, then require the tag to equal the bindings version.
        Prints (and optionally appends to $GITHUB_OUTPUT) the values the
        release workflow needs.

    scripts/release-versions.py require-rc --track rust
        Fail unless the given track's version is a release candidate.

The tag names a *bindings* release. Rust crates are tagged `rust-vX.Y.Z` and
released by .github/workflows/release-rust.yml; see docs/RELEASING.md.

Release policy: only release candidates (`X.Y.Z-rc.N`) may be published, on
every registry. `resolve` enforces this for the bindings and UniFFI tracks,
`require-rc` for the Rust track, and `bump` refuses to write a non-rc version.
There is deliberately no flag to bypass it; publishing a stable version
requires changing RC_ONLY below in a reviewed PR.

Principle for the file lists below: what the tool can locate structurally it
rewrites itself; what a generator produces it only checks and tells you how to
regenerate. Never patch generated files with a regex.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Publishing policy: release candidates only. See the module docstring.
RC_ONLY = True
RC = re.compile(r"^(\d+\.\d+\.\d+)-rc\.(\d+)$")

SEMVER = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta|rc)\.(\d+))?$")
PEP440_PRE = {"alpha": "a", "beta": "b", "rc": "rc"}

TRACKS = ("bindings", "rust", "uniffi")
# The manifest each track's version is read from when a single value is needed.
TRACK_REF = {"bindings": "js/package.json", "rust": "core/Cargo.toml", "uniffi": "uniffi/Cargo.toml"}


def read(path: str) -> str:
    with (ROOT / path).open(newline="") as fh:  # keep the file's own line endings
        return fh.read()


def write(path: str, text: str) -> None:
    with (ROOT / path).open("w", newline="") as fh:
        fh.write(text)


def toml(path: str) -> dict:
    return tomllib.loads(read(path))


def semver_to_pep440(v: str) -> str:
    m = SEMVER.match(v)
    if not m:
        raise ValueError(f"unsupported version {v!r} (X.Y.Z or X.Y.Z-{{alpha,beta,rc}}.N)")
    base = ".".join(m.group(i) for i in (1, 2, 3))
    return base if not m.group(4) else f"{base}{PEP440_PRE[m.group(4)]}{m.group(5)}"


# --------------------------------------------------------------------------- #
# Manifests: the nine files that *define* the three tracks.
# --------------------------------------------------------------------------- #


def collect() -> dict[str, dict[str, str]]:
    workspace = toml("Cargo.toml")
    csproj = read("bindings/csharp/MarketdataUniffi/MarketdataUniffi.csproj")
    gradle = read("bindings/java/build.gradle.kts")
    cs = re.search(r"<Version>([^<]+)</Version>", csproj)
    java = re.search(r'gradleProperty\("projectVersion"\)\.getOrElse\("([^"]+)"\)', gradle)
    return {
        "bindings": {
            "js/package.json": json.loads(read("js/package.json"))["version"],
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
    return [
        f"[{t}] {TRACK_REF[t]} = {versions[t][TRACK_REF[t]]!r} is not a release candidate (X.Y.Z-rc.N). "
        "Only release candidates may be published."
        for t in tracks
        if not RC.match(versions[t][TRACK_REF[t]])
    ]


def track_versions(versions: dict[str, dict[str, str]]) -> dict[str, str]:
    return {t: versions[t][TRACK_REF[t]] for t in TRACKS}


# --------------------------------------------------------------------------- #
# Version slots in text files.
#
# A Slot is one regex whose single capture group is a version string. It is
# used both to *read* a location (check) and to *rewrite* it (bump). Every
# slot must match at least once; a slot that stops matching means the file was
# restructured and this table needs updating, which is reported as an error
# rather than silently skipped.
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class Slot:
    path: str  # repo-relative
    label: str  # shown in output
    pattern: str  # one capture group = the version; compiled with re.MULTILINE
    track: str
    pep440: bool = False  # the slot spells the version the PyPI way
    table: str | None = None  # TOML table the pattern is confined to
    first_line_only: bool = False  # only the file's first line is inspected

    def regex(self) -> re.Pattern:
        return re.compile(self.pattern, re.MULTILINE)

    def expected(self, tracks: dict[str, str]) -> str:
        v = tracks[self.track]
        return semver_to_pep440(v) if self.pep440 else v


def _toml_table_span(text: str, table: str) -> tuple[int, int]:
    """Character span of the body of `[table]` (header excluded, up to the next header)."""
    header = re.search(rf"^\[{re.escape(table)}\]\s*$", text, re.MULTILINE)
    if not header:
        raise ValueError(f"TOML table [{table}] not found")
    nxt = re.search(r"^\[", text[header.end():], re.MULTILINE)
    end = header.end() + nxt.start() if nxt else len(text)
    return header.end(), end


def _slot_region(text: str, slot: Slot) -> tuple[int, int]:
    if slot.first_line_only:
        return 0, len(text.split("\n", 1)[0])
    if slot.table:
        return _toml_table_span(text, slot.table)
    return 0, len(text)


def slot_values(text: str, slot: Slot) -> list[str]:
    start, end = _slot_region(text, slot)
    return [m.group(1) for m in slot.regex().finditer(text, start, end)]


def slot_replace(text: str, slot: Slot, new: str) -> str:
    start, end = _slot_region(text, slot)
    out, pos = [], start
    for m in slot.regex().finditer(text, start, end):
        out.append(text[pos:m.start(1)])
        out.append(new)
        pos = m.end(1)
    out.append(text[pos:end])
    return text[:start] + "".join(out) + text[end:]


# The write side of collect(). Same nine locations, addressed by pattern so
# bump can rewrite them; collect() re-reads them structurally afterwards.
MANIFEST_SLOTS = [
    Slot("js/package.json", "js/package.json", r'^  "version": "([^"]+)",$', "bindings"),
    Slot("Cargo.toml", "Cargo.toml [workspace.package]", r'^version = "([^"]+)"$', "bindings", table="workspace.package"),
    Slot("py/pyproject.toml", "py/pyproject.toml (PEP 440)", r'^version = "([^"]+)"$', "bindings", pep440=True, table="project"),
    Slot("core/Cargo.toml", "core/Cargo.toml", r'^version = "([^"]+)"$', "rust", table="package"),
    Slot("rust/Cargo.toml", "rust/Cargo.toml", r'^version = "([^"]+)"$', "rust", table="package"),
    Slot(
        "Cargo.toml",
        "Cargo.toml marketdata-core alias",
        r'^marketdata-core = \{.*\bversion = "([^"]+)".*\}$',
        "rust",
        table="workspace.dependencies",
    ),
    Slot("uniffi/Cargo.toml", "uniffi/Cargo.toml", r'^version = "([^"]+)"$', "uniffi", table="package"),
    Slot(
        "bindings/csharp/MarketdataUniffi/MarketdataUniffi.csproj",
        "MarketdataUniffi.csproj <Version>",
        r"<Version>([^<]+)</Version>",
        "uniffi",
    ),
    Slot(
        "bindings/java/build.gradle.kts",
        "build.gradle.kts projectVersion default",
        r'gradleProperty\("projectVersion"\)\.getOrElse\("([^"]+)"\)',
        "uniffi",
    ),
]

# Hand-written docs that quote a version at a fixed place. bump rewrites these
# too; check reports drift and points at the file. VER is the character class
# of a version string, so trailing punctuation (a closing backtick, a period)
# stays outside the capture group and survives a rewrite.
VER = r"[0-9A-Za-z.\-]+"
DOC_SLOTS = [
    Slot("docs/INSTALL.md", "INSTALL.md track table, Bindings", r"^\| Bindings \|[^|]*\| `([^`]+)` \(PyPI", "bindings"),
    Slot("docs/INSTALL.md", "INSTALL.md track table, Bindings (PEP 440)", r"^\| Bindings \|.*\(PyPI spells it `([^`]+)`\)", "bindings", pep440=True),
    Slot("docs/INSTALL.md", "INSTALL.md track table, UniFFI", r"^\| UniFFI \|[^|]*\| `([^`]+)` \|$", "uniffi"),
    Slot("docs/INSTALL.md", "INSTALL.md track table, Rust crates", r"^\| Rust crates \|[^|]*\| `([^`]+)` \|$", "rust"),
    Slot("docs/INSTALL.md", "INSTALL.md fugle-marketdata==<PEP 440>", rf"fugle-marketdata==({VER})", "bindings", pep440=True),
    Slot("docs/INSTALL.md", "INSTALL.md go get ...@v<uniffi>", rf"fugle-marketdata-go@v({VER})", "uniffi"),
    Slot("docs/INSTALL.md", "INSTALL.md VERSION=<uniffi>", rf"^VERSION=({VER})", "uniffi"),
    Slot("docs/INSTALL.md", "INSTALL.md TAG=v<bindings>", rf"^TAG=v({VER})", "bindings"),
    # MIGRATION-0.9.md: ONLY the title line carries the current versions.
    # The body is a historical narrative that names old versions on purpose
    # ("3.0.0-rc.1 to rc.4 dropped all of these", "examples that rc.4 silently
    # mishandled", ...). Those mentions are correct as written and must never
    # be flagged or rewritten, so these slots are pinned to line 1 via
    # first_line_only. Do not widen them to the whole file.
    Slot("MIGRATION-0.9.md", "MIGRATION-0.9.md title, bindings", r"^# Migrating to 0\.9\.0 \(bindings ([^,]+), uniffi [^)]+\)$", "bindings", first_line_only=True),
    Slot("MIGRATION-0.9.md", "MIGRATION-0.9.md title, uniffi", r"^# Migrating to 0\.9\.0 \(bindings [^,]+, uniffi ([^)]+)\)$", "uniffi", first_line_only=True),
]


@dataclass(frozen=True)
class Derived:
    """One derived location: what it says, what it should say, how to fix it."""

    track: str
    label: str
    found: list[str]  # every version the location carries ([] = pattern not found)
    expected: str
    fix: str  # command or edit that makes it agree

    @property
    def ok(self) -> bool:
        return bool(self.found) and set(self.found) == {self.expected}

    def error(self) -> str:
        if not self.found:
            return f"[{self.track}] {self.label}: no version found where one was expected. {self.fix}"
        got = ", ".join(sorted(set(self.found)))
        n = f" ({len(self.found)} sites)" if len(self.found) > 1 else ""
        return f"[{self.track}] {self.label} = {got!r}{n}, expected {self.expected!r}. {self.fix}"


def derived_docs(tracks: dict[str, str]) -> list[Derived]:
    texts = {p: read(p) for p in {s.path for s in DOC_SLOTS}}
    return [
        Derived(s.track, s.label, slot_values(texts[s.path], s), s.expected(tracks), f"Edit {s.path} by hand.")
        for s in DOC_SLOTS
    ]


def derived_generated(tracks: dict[str, str]) -> list[Derived]:
    """Generated files. Checked only; the fix is always to re-run the generator."""
    out = []

    # js/index.js: napi emits a `bindingPackageVersion !== '<version>'` check and
    # an `expected <version> but got` message for every platform package.
    index_js = read("js/index.js")
    found = re.findall(r"bindingPackageVersion !== '([^']+)'", index_js)
    found += re.findall(r"expected (\S+) but got ", index_js)
    out.append(
        Derived(
            "bindings",
            "js/index.js napi version check",
            found,
            tracks["bindings"],
            "Run `npm run build:debug` (in js/) to regenerate js/index.js.",
        )
    )

    # js/package-lock.json: root package version, twice.
    lock = json.loads(read("js/package-lock.json"))
    found = [lock.get("version", "<missing>"), lock.get("packages", {}).get("", {}).get("version", "<missing>")]
    out.append(
        Derived(
            "bindings",
            "js/package-lock.json root version",
            found,
            tracks["bindings"],
            "Run `npm install --package-lock-only` (in js/) to refresh js/package-lock.json.",
        )
    )

    # Cargo.lock: one [[package]] entry per workspace crate.
    lock_pkgs = {p["name"]: p["version"] for p in toml("Cargo.lock").get("package", [])}
    crate_tracks = {"core": "rust", "rust": "rust", "uniffi": "uniffi", "py": "bindings", "js": "bindings"}
    for crate_dir, track in crate_tracks.items():
        name = toml(f"{crate_dir}/Cargo.toml")["package"]["name"]
        out.append(
            Derived(
                track,
                f"Cargo.lock {name}",
                [lock_pkgs[name]] if name in lock_pkgs else [],
                tracks[track],
                "Run `cargo update --workspace` to refresh Cargo.lock.",
            )
        )
    return out


def derived_all(tracks: dict[str, str]) -> list[Derived]:
    return derived_docs(tracks) + derived_generated(tracks)


def print_versions(versions: dict[str, dict[str, str]], derived: list[Derived]) -> None:
    for track, sources in versions.items():
        for name, value in sources.items():
            print(f"{track:<9} {name:<42} {value}")
    for d in derived:
        shown = ", ".join(sorted(set(d.found))) if d.found else "<missing>"
        if len(d.found) > 1 and len(set(d.found)) == 1:
            shown += f" ({len(d.found)} sites)"
        print(f"derived   {d.label:<42} {shown}")


def full_check() -> tuple[dict[str, dict[str, str]], list[str]]:
    """Manifest check, then the derived locations. Prints the table; returns errors."""
    versions = collect()
    errors = check(versions)
    derived: list[Derived] = []
    if not errors:
        # The manifests agree, so the track versions are well defined and the
        # derived locations can be compared against them.
        try:
            derived = derived_all(track_versions(versions))
        except FileNotFoundError as e:
            return versions, [f"{Path(e.filename).relative_to(ROOT)} is missing; it is a derived version location"]
        except (tomllib.TOMLDecodeError, json.JSONDecodeError, KeyError) as e:
            return versions, [f"a derived version location could not be parsed ({type(e).__name__}: {e}); regenerate it"]
        errors = [d.error() for d in derived if not d.ok]
    print_versions(versions, derived)
    return versions, errors


# --------------------------------------------------------------------------- #
# bump
# --------------------------------------------------------------------------- #


def next_version(track: str, current: str, requested: str) -> str:
    """Resolve `rc` or an explicit version for one track; raises ValueError."""
    if requested == "rc":
        m = RC.match(current)
        if not m:
            raise ValueError(f"[{track}] current version {current!r} is not an rc; pass an explicit version")
        return f"{m.group(1)}-rc.{int(m.group(2)) + 1}"
    if not SEMVER.match(requested):
        raise ValueError(f"[{track}] {requested!r} is not a supported SemVer version (X.Y.Z or X.Y.Z-{{alpha,beta,rc}}.N)")
    if RC_ONLY and not RC.match(requested):
        raise ValueError(
            f"[{track}] {requested!r} is not a release candidate (X.Y.Z-rc.N). "
            "Only release candidates may be published; bump refuses to write anything else."
        )
    return requested


@dataclass(frozen=True)
class Edit:
    path: str
    label: str
    old: list[str]
    new: str


def plan_edits(targets: dict[str, str], current: dict[str, str]) -> tuple[dict[str, str], list[Edit]]:
    """Apply every slot of a bumped track to in-memory copies of the files."""
    new_tracks = {**current, **targets}
    texts: dict[str, str] = {}
    edits: list[Edit] = []
    for slot in MANIFEST_SLOTS + DOC_SLOTS:
        if slot.track not in targets:
            continue
        text = texts.setdefault(slot.path, read(slot.path))
        old = slot_values(text, slot)
        if not old:
            raise ValueError(f"{slot.path}: {slot.label} not found; update the slot table in {Path(__file__).name}")
        new = slot.expected(new_tracks)
        texts[slot.path] = slot_replace(text, slot, new)
        edits.append(Edit(slot.path, slot.label, old, new))
    return texts, edits


def next_steps(targets: dict[str, str]) -> list[str]:
    steps = []
    if "bindings" in targets:
        steps.append("cd js && npm run build:debug            # regenerates js/index.js (napi embeds the version per platform)")
        steps.append("cd js && npm install --package-lock-only  # refreshes js/package-lock.json")
    steps.append("cargo update --workspace                  # refreshes Cargo.lock for the workspace crates")
    steps.append("Write the CHANGELOG.md entry by hand.")
    steps.append("python3 scripts/release-versions.py check  # must pass before tagging")
    return steps


def bump(args: argparse.Namespace) -> int:
    versions = collect()
    manifest_errors = check(versions)
    if manifest_errors:
        for e in manifest_errors:
            print(f"::error::{e}")
        print("Manifests disagree; fix them (or `git checkout` them) before bumping.")
        return 1
    current = track_versions(versions)

    targets: dict[str, str] = {}
    try:
        for track in TRACKS:
            requested = getattr(args, track)
            if requested is None:
                continue
            new = next_version(track, current[track], requested)
            if new != current[track]:
                targets[track] = new
            else:
                print(f"{track}: already {new}, nothing to do")
    except ValueError as e:
        print(f"::error::{e}")
        return 1
    if not targets:
        if any(getattr(args, t) is not None for t in TRACKS):
            print("Nothing to bump; every requested track is already at its target.")
        else:
            print("Nothing to bump (pass --bindings/--rust/--uniffi with a version or `rc`).")
        return 0

    try:
        texts, edits = plan_edits(targets, current)
    except ValueError as e:
        print(f"::error::{e}")
        return 1

    for track, new in targets.items():
        print(f"{track:<9} {current[track]} -> {new}")
    print()
    for e in edits:
        old = ", ".join(sorted(set(e.old)))
        n = f" ({len(e.old)} sites)" if len(e.old) > 1 else ""
        print(f"  {e.label:<44} {old} -> {e.new}{n}")
    print()

    if args.dry_run:
        print("Dry run: nothing written.")
    else:
        # Snapshot, write, verify, and roll everything back on any disagreement.
        snapshot = {p: read(p) for p in texts}
        interrupted: BaseException | None = None
        try:
            for p, t in texts.items():
                write(p, t)
            after = collect()
            errors = check(after)
            if not errors:
                got = track_versions(after)
                expected = {**current, **targets}
                errors += [f"[{t}] manifests read back {got[t]!r}, expected {expected[t]!r}" for t in TRACKS if got[t] != expected[t]]
                errors += [d.error() for d in derived_docs(got) if not d.ok]
        except Exception as e:  # noqa: BLE001 - anything wrong means roll back
            errors = [f"{type(e).__name__}: {e}"]
        except BaseException as e:  # Ctrl-C mid-write must not leave half the files bumped
            errors, interrupted = ["interrupted"], e
        if errors:
            for p, t in snapshot.items():
                write(p, t)
            if interrupted is not None:
                print("Interrupted; every file has been restored.")
                raise interrupted
            for e in errors:
                print(f"::error::{e}")
            print("Post-bump check failed; every file has been restored.")
            return 1
        print(f"Wrote {len(texts)} files; manifests and docs agree.")

    print()
    print("Next steps (not done by this tool; generated files are only checked, never edited):")
    for i, step in enumerate(next_steps(targets), 1):
        print(f"  {i}. {step}")
    return 0


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    b = sub.add_parser("bump")
    for track in TRACKS:
        b.add_argument(f"--{track}", metavar="VERSION|rc", help=f"new {track} version, or `rc` for rc number + 1")
    b.add_argument("--dry-run", action="store_true", help="print the planned edits without writing")
    r = sub.add_parser("resolve")
    r.add_argument("--tag", required=True)
    r.add_argument("--github-output")
    rc = sub.add_parser("require-rc")
    rc.add_argument("--track", required=True, choices=list(TRACKS), action="append")
    args = ap.parse_args()

    if args.cmd == "bump":
        return bump(args)

    versions, errors = full_check()
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
