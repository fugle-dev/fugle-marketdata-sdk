#!/usr/bin/env python3
"""Verify that every check runs when it should.

Four times a check existed but sat out the change it was meant to catch:
`public-api.yml` listed paths by hand and missed `core/src/errors.rs` (#148);
jest's `testMatch` took `.test.js` only, so `config.test.ts` never ran (#170);
`cargo test -p ...` named three of five workspace crates (#181); and
`version-check.yml` filtered on nine manifests while the script read fourteen
files (#184). Each time the fix was a one-off edit to the list that had
drifted. This script checks the lists against what they are supposed to
cover, so the drift is reported by CI instead of found by accident.

Every expectation is derived from a real source (the workspace manifest, the
files git tracks, the runner's own configuration, the workflow files, what a
script declares it reads). There is no second list to keep in sync here.

    scripts/check-ci-coverage.py            # exit 1 with every gap listed

"Pull-request workflow" below means one that runs on `pull_request`, or one
that such a workflow runs as a job (`uses: ./.github/workflows/x.yml`).

Rules
-----
cargo    every `[workspace] member` is named by a `cargo test -p` (or run
         under `--workspace`) in a pull-request workflow.
jest     every `js/tests/**/*.test.*` file matches jest.config.js `testMatch`.
         Files a pull-request workflow's jest command then skips with
         `--testPathIgnorePatterns` are printed as notes, not errors: that
         exclusion is visible in the workflow, the `testMatch` one was not.
pytest   every `.py` under pytest's `testpaths` that defines tests matches
         `python_files`. Files the pytest command skips with `--ignore` are
         printed as notes, as for jest.
scripts  every `scripts/test_*.py` is invoked by a pull-request workflow.
paths    a workflow with an `on.<event>.paths` filter covers everything its
         steps read: the workflow file itself; every tracked file, directory
         or glob named literally in a `run:` (resolved against the step's
         `working-directory`); `src/**`, `Cargo.toml` and `build.rs` of every
         crate a `cargo ... -p <crate>` names, and of its path dependencies;
         and, for a Python script under scripts/ that it runs, the files that
         script prints for `<script> inputs`. A script that does not answer
         `inputs` cannot be run from a path-filtered workflow.

Glob patterns are read the way GitHub `paths` and jest's micromatch read them
(`**` crosses directories, `*` and `?` do not, `[..]` is a class, a leading
`**/` also matches zero directories, the last matching `paths` entry wins and
`!` negates). Extglobs such as `?(x)` or `+(a|b)` are not supported and are
reported instead of guessed.

Needs PyYAML (`pip install pyyaml`) and `node` on PATH.
"""
from __future__ import annotations

import fnmatch
import glob
import json
import os
import re
import shlex
import subprocess
import sys
import tomllib
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    sys.exit("check-ci-coverage: PyYAML is required (pip install pyyaml)")

ROOT = Path(__file__).resolve().parent.parent
WORKFLOWS = ".github/workflows"


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def tracked(prefix: str) -> list[str]:
    """Repo-relative paths git tracks under `prefix` (build output excluded)."""
    out = subprocess.run(["git", "ls-files", "--", prefix or "."], cwd=ROOT, capture_output=True, text=True, check=True)
    return out.stdout.split()


def note(text: str) -> None:
    print(f"::notice::{text}" if "GITHUB_ACTIONS" in os.environ else f"note: {text}")


# --------------------------------------------------------------------------- #
# Globs
# --------------------------------------------------------------------------- #

EXTGLOB = re.compile(r"[?*+@!]\(")


def glob_to_regex(pattern: str) -> re.Pattern:
    if EXTGLOB.search(pattern):
        raise ValueError(f"extglob pattern {pattern!r} is not supported")
    out, i = [], 0
    while i < len(pattern):
        c = pattern[i]
        if pattern.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        elif c == "?":
            out.append("[^/]")
            i += 1
        elif c == "[":
            j = pattern.find("]", i)
            if j == -1:
                out.append(re.escape(c))
                i += 1
            else:
                out.append(pattern[i : j + 1])
                i = j + 1
        else:
            out.append(re.escape(c))
            i += 1
    return re.compile("^" + "".join(out) + "$")


def covered(path: str, patterns: list[str]) -> bool:
    """GitHub semantics: the last matching pattern wins; `!` negates."""
    result = False
    for p in patterns:
        negate = p.startswith("!")
        if glob_to_regex(p.lstrip("!")).match(path):
            result = not negate
    return result


# --------------------------------------------------------------------------- #
# Workflows
# --------------------------------------------------------------------------- #


def shell_words(line: str) -> list[str]:
    """One command line as shell words, with `;`, `|`, `&`, `(`, `)` split off."""
    lex = shlex.shlex(line, posix=True, punctuation_chars=True)
    lex.whitespace_split = True
    lex.commenters = "#"
    try:
        return list(lex)
    except ValueError:  # an unbalanced quote, e.g. inside a heredoc
        return line.split()


class Workflow:
    def __init__(self, path: Path):
        self.path = rel(path)
        self.doc = yaml.safe_load(path.read_text()) or {}
        # PyYAML reads the bare key `on` as boolean True.
        self.on = self.doc.get("on", self.doc.get(True, {}))
        if isinstance(self.on, (str, list)):
            self.on = {e: None for e in ([self.on] if isinstance(self.on, str) else self.on)}

    def paths(self, event: str) -> list[str] | None:
        cfg = self.on.get(event)
        return cfg.get("paths") if isinstance(cfg, dict) else None

    def calls(self) -> set[str]:
        """Reusable workflows this one runs as jobs (`uses: ./.github/workflows/x.yml`)."""
        return {
            job["uses"].removeprefix("./")
            for job in (self.doc.get("jobs") or {}).values()
            if isinstance(job.get("uses"), str) and job["uses"].startswith("./")
        }

    def commands(self) -> list[tuple[str, list[str]]]:
        """Each command line of each `run:` step as (working-directory, words)."""
        out = []
        for job in (self.doc.get("jobs") or {}).values():
            job_cwd = ((job.get("defaults") or {}).get("run") or {}).get("working-directory", "")
            for step in job.get("steps") or []:
                if not isinstance(step.get("run"), str):
                    continue
                cwd = step.get("working-directory", job_cwd).strip("/")
                for line in step["run"].replace("\\\n", " ").splitlines():
                    words = shell_words(line.strip())
                    if words:
                        out.append((cwd, words))
        return out


def load_workflows() -> list[Workflow]:
    return [Workflow(p) for p in sorted((ROOT / WORKFLOWS).glob("*.yml"))]


def pull_request_gate(workflows: list[Workflow]) -> list[Workflow]:
    """Workflows that run on pull requests, directly or as a called workflow."""
    gating = {wf.path for wf in workflows if "pull_request" in wf.on}
    while True:
        called = {c for wf in workflows if wf.path in gating for c in wf.calls()}
        if called <= gating:
            return [wf for wf in workflows if wf.path in gating]
        gating |= called


def option_values(words: list[str], option: str) -> list[str]:
    """Values of `--option X` and `--option=X` in a word list."""
    out = []
    for i, w in enumerate(words):
        if w == option and i + 1 < len(words):
            out.append(words[i + 1])
        elif w.startswith(option + "="):
            out.append(w.split("=", 1)[1])
    return out


def cargo_packages(words: list[str], subcommand: str | None = None) -> set[str] | None:
    """Crate names a `cargo [+toolchain] <subcommand> ... -p X` line names.

    Returns None when the line is not such a cargo invocation, and the sentinel
    {"*"} for `--workspace` / `--all`.
    """
    if "cargo" not in words:
        return None
    rest = words[words.index("cargo") + 1 :]
    if rest and rest[0].startswith("+"):
        rest = rest[1:]
    if not rest or (subcommand and rest[0] != subcommand):
        return None
    pkgs = set(option_values(rest, "-p")) | set(option_values(rest, "--package"))
    if "--workspace" in rest or "--all" in rest:
        pkgs.add("*")
    return pkgs


# --------------------------------------------------------------------------- #
# Cargo workspace
# --------------------------------------------------------------------------- #


def workspace_members() -> dict[str, str]:
    """crate name -> crate directory, from `[workspace] members` (globs expanded)."""
    members = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["members"]
    out = {}
    for m in members:
        for d in sorted(glob.glob(m, root_dir=ROOT)):
            manifest = ROOT / d / "Cargo.toml"
            if manifest.is_file():
                out[tomllib.loads(manifest.read_text())["package"]["name"]] = d
    return out


def crate_inputs(crate_dir: str, seen: set[str] | None = None) -> set[str]:
    """Files that shape a crate's build: its sources, manifest and build
    script, and the same for every path dependency (direct or via the
    workspace table)."""
    seen = seen if seen is not None else set()
    if crate_dir in seen:
        return set()
    seen.add(crate_dir)
    files = set(tracked(f"{crate_dir}/src"))
    files |= {f for f in (f"{crate_dir}/Cargo.toml", f"{crate_dir}/build.rs") if (ROOT / f).is_file()}
    manifest = tomllib.loads((ROOT / crate_dir / "Cargo.toml").read_text())
    workspace_deps = tomllib.loads((ROOT / "Cargo.toml").read_text()).get("workspace", {}).get("dependencies", {})
    for table in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, spec in manifest.get(table, {}).items():
            if isinstance(spec, dict) and spec.get("workspace"):
                spec = workspace_deps.get(name, {})
                base = ROOT  # a workspace path is relative to the root manifest
            else:
                base = ROOT / crate_dir
            if isinstance(spec, dict) and "path" in spec:
                files |= crate_inputs(rel((base / spec["path"]).resolve()), seen)
    return files


# --------------------------------------------------------------------------- #
# Rules
# --------------------------------------------------------------------------- #


def check_cargo(workflows: list[Workflow]) -> list[str]:
    members = workspace_members()
    tested: set[str] = set()
    for wf in pull_request_gate(workflows):
        for _, words in wf.commands():
            pkgs = cargo_packages(words, "test")
            if pkgs:
                tested |= members.keys() if "*" in pkgs else pkgs
    return [
        f"[cargo] workspace member `{name}` ({d}/) is not named by any `cargo test -p` in a pull-request workflow"
        for name, d in members.items()
        if name not in tested
    ]


def jest_config() -> dict:
    out = subprocess.run(
        ["node", "-e", "process.stdout.write(JSON.stringify(require(process.argv[1])))", "./jest.config.js"],
        cwd=ROOT / "js",
        capture_output=True,
        text=True,
    )
    if out.returncode != 0:
        raise RuntimeError(f"could not load js/jest.config.js with node: {out.stderr.strip()}")
    return json.loads(out.stdout)


def check_jest(workflows: list[Workflow]) -> list[str]:
    cfg = jest_config()
    test_match = cfg.get("testMatch", ["**/__tests__/**/*.[jt]s?(x)", "**/?(*.)+(spec|test).[jt]s?(x)"])
    test_regex = cfg.get("testRegex", [])
    test_regex = [test_regex] if isinstance(test_regex, str) else test_regex
    ignore = [re.compile(p) for p in cfg.get("testPathIgnorePatterns", ["/node_modules/"])]
    cli_ignore = [
        re.compile(p)
        for wf in pull_request_gate(workflows)
        for _, words in wf.commands()
        if any(w == "jest" or w.endswith("/jest") for w in words)
        for p in option_values(words, "--testPathIgnorePatterns")
    ]
    errors, skipped = [], []
    for f in tracked("js/tests"):
        name = Path(f).name
        if ".test." not in name and ".spec." not in name:
            continue
        p = f.removeprefix("js/")  # jest matches the absolute path; `**/` absorbs the prefix
        if any(r.search("/" + p) for r in ignore):
            continue
        if not any(glob_to_regex(g).match(p) for g in test_match) and not any(re.search(r, p) for r in test_regex):
            errors.append(f"[jest] {f} is a test file but matches no testMatch pattern {test_match} in js/jest.config.js")
        elif any(r.search("/" + p) for r in cli_ignore):
            skipped.append(f)
    # A note, not an error: an exclusion on the command line is visible in
    # the workflow and reviewed with it, unlike a file that testMatch quietly
    # never picked up. Failing here would only add noise to a deliberate
    # choice (integration tests need credentials).
    if skipped:
        note(f"[jest] {len(skipped)} test file(s) are excluded by --testPathIgnorePatterns on the jest command in CI: " + ", ".join(skipped))
    return errors


DEFINES_TESTS = re.compile(r"^(?:async\s+)?def test_|^class Test", re.MULTILINE)


def check_pytest(workflows: list[Workflow]) -> list[str]:
    ini = tomllib.loads((ROOT / "py/pyproject.toml").read_text()).get("tool", {}).get("pytest", {}).get("ini_options", {})
    testpaths = ini.get("testpaths", ["."])
    python_files = ini.get("python_files", "test_*.py *_test.py")
    python_files = python_files.split() if isinstance(python_files, str) else python_files
    cli_ignore = {
        (Path(cwd) / p).as_posix().removeprefix("./")
        for wf in pull_request_gate(workflows)
        for cwd, words in wf.commands()
        if "pytest" in words
        for p in option_values(words, "--ignore")
    }
    errors, skipped = [], []
    for tp in testpaths:
        for f in tracked(f"py/{tp}"):
            name = Path(f).name
            if not name.endswith(".py") or name in ("conftest.py", "__init__.py"):
                continue
            if not DEFINES_TESTS.search((ROOT / f).read_text()):
                continue
            if not any(fnmatch.fnmatch(name, pat) for pat in python_files):
                errors.append(f"[pytest] {f} defines tests but matches no python_files pattern {python_files} in py/pyproject.toml")
            elif any(f == i or f.startswith(i + "/") for i in cli_ignore):
                skipped.append(f)
    # A note for the same reason as in check_jest: the exclusion is on the
    # command line, where a reviewer sees it.
    if skipped:
        note(f"[pytest] {len(skipped)} test file(s) are excluded by --ignore on the pytest command in CI: " + ", ".join(skipped))
    return errors


def check_scripts_tests(workflows: list[Workflow]) -> list[str]:
    invoked: set[str] = set()
    for wf in pull_request_gate(workflows):
        for cwd, words in wf.commands():
            invoked |= {(Path(cwd) / w).as_posix().removeprefix("./") for w in words}
    return [
        f"[scripts] {f} is not invoked by any pull-request workflow"
        for f in tracked("scripts")
        if Path(f).name.startswith("test_") and f.endswith(".py") and f not in invoked
    ]


def script_inputs(script: str) -> list[str] | str:
    """What `<script> inputs` prints, or an error message."""
    out = subprocess.run([sys.executable, script, "inputs"], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        return f"`{script} inputs` failed ({out.stderr.strip() or out.stdout.strip()})"
    return out.stdout.split()


def workflow_inputs(wf: Workflow, members: dict[str, str]) -> tuple[list[str], list[str]]:
    """Files a workflow's steps read, and errors met while deriving them."""
    inputs, errors = {wf.path}, []
    all_tracked = set(tracked(""))
    for cwd, words in wf.commands():
        for w in words:
            for token in w.split("="):
                token = token.strip("'\"")
                if not token or token.startswith(("-", "$", "/", "~")) or ".." in token:
                    continue
                token = (Path(cwd) / token).as_posix().removeprefix("./")
                if "*" in token or "?" in token:
                    inputs |= set(glob.glob(token, root_dir=ROOT, recursive=True)) & all_tracked
                elif token in all_tracked:
                    inputs.add(token)
                    if token.startswith("scripts/") and token.endswith(".py"):
                        declared = script_inputs(token)
                        if isinstance(declared, str):
                            errors.append(f"[paths] {wf.path} runs {token} but {declared}; a script run from a path-filtered workflow must print what it reads for `inputs`")
                        else:
                            inputs |= set(declared)
                elif (ROOT / token).is_dir():
                    inputs |= set(tracked(token))
        for pkg in cargo_packages(words) or ():
            for d in [members[pkg]] if pkg in members else members.values() if pkg == "*" else []:
                inputs |= crate_inputs(d)
    return sorted(inputs), errors


def check_paths(workflows: list[Workflow]) -> list[str]:
    members = workspace_members()
    errors = []
    for wf in workflows:
        filtered = {e: wf.paths(e) for e in ("pull_request", "push") if wf.paths(e)}
        if not filtered:
            continue
        inputs, derive_errors = workflow_inputs(wf, members)
        errors += derive_errors
        for event, patterns in filtered.items():
            missing = [f for f in inputs if not covered(f, patterns)]
            if missing:
                errors.append(
                    f"[paths] {wf.path} on.{event}.paths does not cover {len(missing)} file(s) its steps read: "
                    + ", ".join(missing[:8])
                    + (f", ... ({len(missing) - 8} more)" if len(missing) > 8 else "")
                )
    return errors


def main() -> int:
    workflows = load_workflows()
    rules = {
        "cargo": check_cargo,
        "jest": check_jest,
        "pytest": check_pytest,
        "scripts": check_scripts_tests,
        "paths": check_paths,
    }
    errors = []
    for name, rule in rules.items():
        try:
            errors += rule(workflows)
        except Exception as e:  # a rule that cannot run is a gap, not a pass
            errors.append(f"[{name}] could not run: {type(e).__name__}: {e}")
    for e in errors:
        print(f"::error::{e}" if "GITHUB_ACTIONS" in os.environ else f"error: {e}")
    if errors:
        return 1
    print("check-ci-coverage: every check covers what it should")
    return 0


if __name__ == "__main__":
    sys.exit(main())
