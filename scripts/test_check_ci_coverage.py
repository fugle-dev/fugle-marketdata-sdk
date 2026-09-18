#!/usr/bin/env python3
"""Tests for scripts/check-ci-coverage.py.

    python3 scripts/test_check_ci_coverage.py

Each test builds a throwaway repository skeleton in a temp directory and
points the script's ROOT at it. `tracked()` is replaced by a directory walk
so the skeleton needs no git repository. One test per historical gap (#148,
#170, #181, #184 and the never-run scripts/test_release_versions.py), plus
the glob semantics the rules rely on.
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().with_name("check-ci-coverage.py")
spec = importlib.util.spec_from_file_location("check_ci_coverage", SCRIPT)
cc = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = cc
spec.loader.exec_module(cc)

WORKSPACE = 'workspace.members = ["core", "py"]\n'
CORE = '[package]\nname = "fugle-marketdata-core"\n'
PY = '[package]\nname = "marketdata-py"\n'
JEST_JS_ONLY = "module.exports = { testMatch: ['**/tests/**/*.test.js'] };\n"
JEST_BOTH = "module.exports = { testMatch: ['**/tests/**/*.test.[jt]s'] };\n"
PYPROJECT = '[tool.pytest.ini_options]\ntestpaths = ["tests"]\n'


def workflow(on: str, run: str, name: str = "ci") -> dict[str, str]:
    body = (
        f"name: {name}\n"
        "on:\n" + textwrap.indent(textwrap.dedent(on), "  ") + "\n"
        "jobs:\n"
        "  job:\n"
        "    runs-on: ubuntu-latest\n"
        "    steps:\n"
        "      - run: |\n" + textwrap.indent(textwrap.dedent(run), "          ") + "\n"
    )
    return {f".github/workflows/{name}.yml": body}


class Case(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name).resolve()
        self.addCleanup(self._tmp.cleanup)
        self.addCleanup(setattr, cc, "ROOT", cc.ROOT)
        cc.ROOT = self.root

        def walk(prefix: str) -> list[str]:
            base = self.root / prefix if prefix else self.root
            if base.is_file():
                return [prefix]
            return sorted(p.relative_to(self.root).as_posix() for p in base.rglob("*") if p.is_file())

        self.addCleanup(setattr, cc, "tracked", cc.tracked)
        cc.tracked = walk

    def write(self, files: dict[str, str]) -> None:
        for rel, text in files.items():
            p = self.root / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)

    def workflows(self):
        return cc.load_workflows()


class TestGlob(unittest.TestCase):
    def test_github_paths_semantics(self):
        m = lambda pattern, path: bool(cc.glob_to_regex(pattern).match(path))
        self.assertTrue(m("core/src/**", "core/src/rest/params.rs"))
        self.assertFalse(m("core/src/*", "core/src/rest/params.rs"))
        self.assertTrue(m("**/*.md", "README.md"))
        self.assertTrue(m("**/*.md", "docs/a/b.md"))
        self.assertFalse(m("*.md", "docs/a.md"))
        self.assertTrue(m("**/tests/**/*.test.[jt]s", "tests/config.test.ts"))
        self.assertFalse(m("**/tests/**/*.test.js", "tests/config.test.ts"))
        self.assertTrue(m("js/tests/**", "js/tests/fixtures/x.json"))

    def test_extglob_is_reported_not_guessed(self):
        with self.assertRaises(ValueError):
            cc.glob_to_regex("**/?(*.)+(spec|test).[jt]s?(x)")

    def test_shell_punctuation_is_split_off(self):
        self.assertEqual(cc.shell_words("(cd js && npm ci); grep -q x a.kts; then"), ["(", "cd", "js", "&&", "npm", "ci", ");", "grep", "-q", "x", "a.kts", ";", "then"])
        self.assertEqual(cc.shell_words("cargo test --locked -p marketdata-py --no-default-features"), ["cargo", "test", "--locked", "-p", "marketdata-py", "--no-default-features"])

    def test_last_match_wins_and_negation(self):
        self.assertTrue(cc.covered("core/src/lib.rs", ["core/**"]))
        self.assertFalse(cc.covered("core/src/lib.rs", ["core/**", "!core/src/**"]))
        self.assertTrue(cc.covered("core/src/lib.rs", ["core/**", "!core/src/**", "core/src/lib.rs"]))
        self.assertFalse(cc.covered("py/src/lib.rs", ["core/**"]))


class TestCargo(Case):
    """#181: a workspace member no `cargo test -p` names."""

    def setUp(self):
        super().setUp()
        self.write({"Cargo.toml": WORKSPACE, "core/Cargo.toml": CORE, "py/Cargo.toml": PY})

    def test_member_left_out_is_reported(self):
        self.write(workflow("pull_request:", "cargo test --locked -p fugle-marketdata-core"))
        errors = cc.check_cargo(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("marketdata-py", errors[0])

    def test_every_member_named_passes(self):
        self.write(workflow("pull_request:", "cargo test -p fugle-marketdata-core\ncargo test -p marketdata-py --no-default-features"))
        self.assertEqual(cc.check_cargo(self.workflows()), [])

    def test_workspace_flag_covers_all(self):
        self.write(workflow("pull_request:", "cargo +stable test --workspace"))
        self.assertEqual(cc.check_cargo(self.workflows()), [])

    def test_manual_workflow_does_not_count(self):
        self.write(workflow("workflow_dispatch:", "cargo test --workspace", name="manual"))
        self.assertEqual(len(cc.check_cargo(self.workflows())), 2)

    def test_called_workflow_counts(self):
        self.write(workflow("pull_request:", "true"))
        self.write({".github/workflows/ci.yml": "name: ci\non:\n  pull_request:\njobs:\n  t:\n    uses: ./.github/workflows/rust.yml\n"})
        self.write(workflow("workflow_call:", "cargo test --workspace", name="rust"))
        self.assertEqual(cc.check_cargo(self.workflows()), [])


class TestJest(Case):
    """#170: a `.test.ts` file that `testMatch` does not match."""

    def setUp(self):
        super().setUp()
        self.write({"js/tests/a.test.js": "", "js/tests/config.test.ts": "", "js/tests/fixtures/quote.json": "{}"})

    def test_unmatched_extension_is_reported(self):
        self.write({"js/jest.config.js": JEST_JS_ONLY})
        errors = cc.check_jest(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("js/tests/config.test.ts", errors[0])

    def test_both_extensions_pass(self):
        self.write({"js/jest.config.js": JEST_BOTH})
        self.assertEqual(cc.check_jest(self.workflows()), [])

    def test_cli_ignore_is_a_note_not_an_error(self):
        self.write({"js/jest.config.js": JEST_BOTH, "js/tests/rest-integration.test.js": ""})
        self.write(workflow("pull_request:", "npx jest --ci --testPathIgnorePatterns=integration"))
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(cc.check_jest(self.workflows()), [])
        self.assertIn("js/tests/rest-integration.test.js", out.getvalue())
        self.assertNotIn("config.test.ts", out.getvalue())

    def test_ignored_paths_are_not_reported(self):
        self.write({"js/jest.config.js": "module.exports = { testMatch: ['**/*.test.js'], testPathIgnorePatterns: ['/tests/config'] };\n"})
        self.assertEqual(cc.check_jest(self.workflows()), [])


class TestPytest(Case):
    def setUp(self):
        super().setUp()
        self.write({"py/pyproject.toml": PYPROJECT, "py/tests/conftest.py": "def test_not_a_test(): pass\n", "py/tests/loopback.py": "def helper(): pass\n"})

    def test_misnamed_test_module_is_reported(self):
        self.write({"py/tests/websocket_checks.py": "def test_x():\n    pass\n"})
        errors = cc.check_pytest(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("py/tests/websocket_checks.py", errors[0])

    def test_conventional_names_and_helpers_pass(self):
        self.write({"py/tests/test_ws.py": "class TestWs:\n    pass\n", "py/tests/ws_test.py": "async def test_y():\n    pass\n"})
        self.assertEqual(cc.check_pytest(self.workflows()), [])

    def test_cli_ignore_is_a_note_not_an_error(self):
        self.write({"py/tests/test_performance.py": "def test_x(): pass\n", "py/tests/test_rest.py": "def test_y(): pass\n"})
        self.write({".github/workflows/ci.yml": "name: ci\non:\n  pull_request:\njobs:\n  py:\n    steps:\n      - working-directory: py\n        run: pytest -q tests --ignore=tests/test_performance.py\n"})
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(cc.check_pytest(self.workflows()), [])
        self.assertIn("py/tests/test_performance.py", out.getvalue())
        self.assertNotIn("test_rest.py", out.getvalue())

    def test_custom_python_files_is_honoured(self):
        self.write({"py/pyproject.toml": PYPROJECT + 'python_files = "check_*.py"\n', "py/tests/check_ws.py": "def test_x(): pass\n"})
        self.assertEqual(cc.check_pytest(self.workflows()), [])


class TestScriptsTests(Case):
    """scripts/test_release_versions.py existed for months and no workflow ran it."""

    def setUp(self):
        super().setUp()
        self.write({"scripts/test_tool.py": "", "scripts/tool.py": ""})

    def test_uninvoked_test_is_reported(self):
        self.write(workflow("pull_request:", "python3 scripts/tool.py check"))
        errors = cc.check_scripts_tests(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("scripts/test_tool.py", errors[0])

    def test_dot_slash_and_working_directory_are_normalised(self):
        self.write({".github/workflows/ci.yml": "name: ci\non:\n  pull_request:\njobs:\n  t:\n    defaults:\n      run:\n        working-directory: scripts\n    steps:\n      - run: python3 ./test_tool.py\n"})
        self.assertEqual(cc.check_scripts_tests(self.workflows()), [])

    def test_invoked_from_called_workflow_passes(self):
        self.write({".github/workflows/ci.yml": "name: ci\non:\n  pull_request:\njobs:\n  t:\n    uses: ./.github/workflows/tool.yml\n"})
        self.write(workflow("workflow_call:", "python3 scripts/test_tool.py", name="tool"))
        self.assertEqual(cc.check_scripts_tests(self.workflows()), [])


class TestPaths(Case):
    """#148 and #184: a `paths` filter narrower than what the job reads."""

    def setUp(self):
        super().setUp()
        self.write({"Cargo.toml": WORKSPACE, "core/Cargo.toml": CORE, "py/Cargo.toml": PY, "core/src/lib.rs": "", "core/src/errors.rs": "", "core/README.md": ""})

    def test_crate_source_outside_paths_is_reported(self):
        # public-api.yml before #148: `core/src/lib.rs` listed by hand.
        self.write(workflow("pull_request:\n  paths:\n    - core/src/lib.rs\n    - .github/workflows/ci.yml", "cargo public-api -p fugle-marketdata-core"))
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("core/src/errors.rs", errors[0])
        self.assertIn("core/Cargo.toml", errors[0])
        self.assertNotIn("core/README.md", errors[0])

    def test_path_dependencies_count_as_inputs(self):
        # `-p fugle-marketdata` (rust/) depends on core/ through the workspace table.
        self.write({
            "Cargo.toml": 'workspace.members = ["core", "rust"]\nworkspace.dependencies.marketdata-core = { path = "core" }\n',
            "rust/Cargo.toml": '[package]\nname = "fugle-marketdata"\n[dependencies]\nmarketdata-core = { workspace = true }\n',
            "rust/src/lib.rs": "",
        })
        self.write(workflow("pull_request:\n  paths:\n    - rust/**\n    - .github/workflows/ci.yml", "cargo test -p fugle-marketdata"))
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("core/src/errors.rs", errors[0])

    def test_tokens_resolve_against_working_directory_and_directories_expand(self):
        self.write({"js/index.d.ts": "", "js/types.d.ts": "", "bindings/cpp/a.hpp": ""})
        self.write({".github/workflows/ci.yml": "name: ci\non:\n  pull_request:\n    paths:\n      - js/index.d.ts\n      - .github/workflows/ci.yml\njobs:\n  t:\n    steps:\n      - working-directory: js\n        run: npx tsc --noEmit index.d.ts types.d.ts\n      - run: ls bindings/cpp\n"})
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("js/types.d.ts", errors[0])
        self.assertIn("bindings/cpp/a.hpp", errors[0])
        self.assertNotIn("js/index.d.ts", errors[0])

    def test_crate_dir_glob_passes(self):
        self.write(workflow("pull_request:\n  paths:\n    - core/**\n    - .github/workflows/ci.yml", "cargo public-api -p fugle-marketdata-core"))
        self.assertEqual(cc.check_paths(self.workflows()), [])

    def test_workflow_must_cover_itself_and_literal_files(self):
        self.write({".markdownlint.json": "{}", "docs/a.md": ""})
        self.write(workflow("pull_request:\n  paths:\n    - '**/*.md'", "markdownlint 'docs/**/*.md' --config .markdownlint.json"))
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn(".markdownlint.json", errors[0])
        self.assertIn(".github/workflows/ci.yml", errors[0])
        self.assertNotIn("docs/a.md", errors[0])

    def test_push_and_pull_request_are_checked_separately(self):
        self.write(workflow("pull_request:\n  paths:\n    - '**'\npush:\n  paths:\n    - core/**", "cargo test -p fugle-marketdata-core"))
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("on.push.paths", errors[0])

    def test_unfiltered_workflow_is_skipped(self):
        self.write(workflow("pull_request:", "cargo test -p fugle-marketdata-core"))
        self.assertEqual(cc.check_paths(self.workflows()), [])

    def test_script_inputs_must_be_covered(self):
        # version-check.yml before #184: the script read docs/INSTALL.md, the
        # paths listed manifests only.
        self.write({"scripts/tool.py": "import sys\nprint('js/package.json\\ndocs/INSTALL.md')\n", "js/package.json": "{}", "docs/INSTALL.md": ""})
        self.write(workflow("pull_request:\n  paths:\n    - js/package.json\n    - scripts/tool.py\n    - .github/workflows/ci.yml", "python3 scripts/tool.py check"))
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("docs/INSTALL.md", errors[0])

    def test_script_without_inputs_is_reported(self):
        self.write({"scripts/tool.py": "import sys\nsys.exit('no such command')\n"})
        self.write(workflow("pull_request:\n  paths:\n    - '**'", "python3 scripts/tool.py check"))
        errors = cc.check_paths(self.workflows())
        self.assertEqual(len(errors), 1)
        self.assertIn("`inputs`", errors[0])


class TestAgainstRealRepo(unittest.TestCase):
    """The rules hold for this repository; a regression is a CI failure here."""

    def test_repo_passes(self):
        if not (cc.ROOT / ".git").exists() or not (cc.ROOT / "js/jest.config.js").exists():
            self.skipTest("not run from the repository")
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(cc.main(), 0)


if __name__ == "__main__":
    unittest.main()
