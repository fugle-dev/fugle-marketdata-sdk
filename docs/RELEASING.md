# Releasing

This repository ships three independently versioned tracks.
`scripts/release-versions.py` is the single source of truth for the rules
below; CI runs it on every pull request.

| Track | Manifests (must agree) | Published as | Tag |
|---|---|---|---|
| Bindings | `js/package.json`, `[workspace.package]` in `Cargo.toml`, `py/pyproject.toml` (PEP 440 spelling) | PyPI `fugle-marketdata`, npm `@fugle/marketdata` | `vX.Y.Z-rc.N` |
| UniFFI | `uniffi/Cargo.toml`, `<Version>` in the `.csproj`, the Gradle `projectVersion` default | NuGet `Fugle.MarketData`, Go `github.com/fugle-dev/fugle-marketdata-go`, C++ tarballs | none of its own |
| Rust crates | `core/Cargo.toml`, `rust/Cargo.toml`, the `marketdata-core` alias in `Cargo.toml` | crates.io `fugle-marketdata-core`, `fugle-marketdata` | `rust-vX.Y.Z-rc.N` |

```bash
python3 scripts/release-versions.py check
```

## Rules

- **Only release candidates (`X.Y.Z-rc.N`) may be published, on every
  registry.** `scripts/release-versions.py` rejects any other bindings, UniFFI
  or Rust version before anything is built, and each publish workflow checks
  again. There is no flag to bypass this; a stable release needs a reviewed
  change to `RC_ONLY` in that script and to the publish workflow guards.

- A `v*` tag releases the **bindings** track and must equal the bindings
  version. The Release workflow rejects anything else.
- Every bindings release also builds and publishes the current UniFFI version.
  Bump the UniFFI version whenever C#, Go or C++ users would see a change,
  including core fixes they inherit. An unchanged UniFFI version is skipped by
  NuGet (`--skip-duplicate`).
- Pre-releases use `-rc.N`. They are published to the real registries on
  channels users never get by default: pip needs `--pre`, npm uses the `next`
  dist-tag, NuGet needs `--prerelease`, Go needs an explicit version.
- The npm workflow refuses to put a pre-release on `latest` or a stable
  version on `next`.
- Each registry is opt-in through a repository variable (Settings → Secrets
  and variables → Actions → Variables): `PUBLISH_PYPI`, `PUBLISH_NPM`,
  `PUBLISH_NUGET`, `PUBLISH_GO` and `PUBLISH_JAVA`, each set to `true`. A tag
  with no registry enabled fails early. Release notes and **Verify Release**
  only cover the enabled registries.
- To publish a registry that was disabled when a version was tagged, enable it
  and re-run that tag's **Release** run. Publish steps skip versions that are
  already on the registry.
- Java is built but not published unless `PUBLISH_JAVA=true`.

## One-time setup

These require organization or registry admin access.

| Item | Where | Value |
|---|---|---|
| Repository visibility | GitHub settings | Public. npm provenance and anonymous `go get` need it. |
| Environment `release` | GitHub settings, Environments | Used by the PyPI job. Add required reviewers if you want a manual gate. |
| PyPI trusted publisher | pypi.org, project `fugle-marketdata`, Publishing | Owner `fugle-dev`, repository `fugle-marketdata-sdk`, workflow `release.yml`, environment `release` |
| npm trusted publisher | npmjs.com, `@fugle/marketdata` and each `@fugle/marketdata-<platform>` package | Repository `fugle-dev/fugle-marketdata-sdk`, workflow `release.yml`. Until the platform packages exist, use the `NPM_TOKEN` secret instead. |
| `NPM_TOKEN` secret | GitHub secrets | Granular automation token with publish rights on `@fugle`. Optional once trusted publishing covers every package. |
| `NUGET_API_KEY` secret | GitHub secrets | nuget.org API key scoped to push `Fugle.MarketData` |
| `GO_REPO_DEPLOY_KEY` secret | GitHub secrets | Private half of a deploy key that has write access to `fugle-dev/fugle-marketdata-go` |
| crates.io trusted publisher | crates.io, each of `fugle-marketdata-core` and `fugle-marketdata`: Settings → Trusted Publishing | Owner `fugle-dev`, repository `fugle-marketdata-sdk`, workflow `release-rust.yml`, environment `release` |

npm validates the **calling** workflow, and PyPI does not accept reusable
workflows, which is why both trusted publishers point at `release.yml`.

## Releasing the Rust crates

The **Release Rust crates** workflow publishes both crates with crates.io
trusted publishing; no token is stored. Rehearse it first with
`gh workflow run release-rust.yml --ref main`, which runs
`cargo publish --dry-run` and a crates.io OIDC login (to check the trusted
publisher configuration) but publishes nothing. Then tag the core version:

```bash
python3 scripts/release-versions.py check
git tag rust-v0.9.0-rc.1
git push origin rust-v0.9.0-rc.1
```

The tag must equal the version in `core/Cargo.toml`. Crates that are already
on crates.io at that version are skipped, so a failed run can be re-run.

## Rehearsing a release

Run the **Release** workflow manually from the Actions tab, or:

```bash
gh workflow run release.yml --ref main
```

A manual run builds every platform and runs each publish job up to the
upload: wheel checks for PyPI, `npm publish --dry-run` for every npm package,
`dotnet pack` for NuGet, and Go module assembly plus a static-link smoke test.
Nothing is published, no tag is needed, and no GitHub Release is created. The
**Rehearsal summary** job fails if any step would have failed. Registry
credentials are not needed for a rehearsal.

## Releasing the bindings

1. Update `CHANGELOG.md` with the release date and make sure CI is green on
   `main`. CI checks that the committed UniFFI bindings match the Rust
   interface. Rehearse the release first if the release workflow or build
   configuration changed since the last release.
2. Tag the bindings version and push the tag:

   ```bash
   git tag v3.0.0-rc.2
   git push origin v3.0.0-rc.2
   ```

3. Watch the **Release** workflow. It builds every platform, publishes to
   PyPI, npm, NuGet and the Go module repository, then creates the GitHub
   Release with the C++ tarballs. The GitHub Release is only created when
   every publish job succeeded.
4. If a publish job fails, fix the cause and re-run the failed jobs. Every
   publish step skips versions that already exist.
5. **Verify Release** starts automatically afterwards. It installs each
   package from its registry on Linux, macOS and Windows and constructs a
   client. It also checks that the npm pre-release did not land on `latest`.
