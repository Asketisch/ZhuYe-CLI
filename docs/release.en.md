# Release Process

<p align="center">  [中文](release.md) · **English**  </p>

> The full pipeline from pre-release review through tagging, packaging, and
> installation. For the development and test environment see
> [development.md](development.en.md); changelog conventions are in
> section 2 below.

## 1. Pre-Release Review

Before a release, run one full review pass covering the **compile surface +
behavior surface + coverage audit**:

```bash
make release-check
```

It runs, in order:

1. `./tests/run_all.sh` — the Rust compile surface (`cargo test` /
   `clippy -D warnings` / `fmt` / `doc`) + the capability coverage layer
   (behavior assertions against the real binary over a pty + the CLI argument
   matrix);
2. `tests/tools/coverage_audit.py --strict` — confirms that every declared
   slash command has a behavior-level test case.

If any of these fails, you **should not release**. Review and packaging are
two steps: `make release-check` first, then package.

## 2. CHANGELOG Conventions (mechanically parsed — do not change the format)

`CHANGELOG.md` follows the Keep a Changelog framework with a Chinese body:

- Unreleased changes go under the `## [Unreleased]` section;
- version headers are fixed as `## [x.y.z] — YYYY-MM-DD` (em-dash separated);
- the empty-section placeholder is fixed as `(无)` ("none", in Chinese);
- at release time `create_tag.sh` **promotes** the `[Unreleased]` contents
  into a new version section — the script parses `CHANGELOG.md`
  **mechanically** against those markers; changing the header language,
  version-header format, or placeholder breaks the release flow outright.

## 3. Tagging (`./scripts/create_tag.sh`)

One-shot release: version bump + CHANGELOG promotion + commit + annotated
tag + push.

```bash
./scripts/create_tag.sh --dry-run    # print only, preferred for verifying the flow
./scripts/create_tag.sh --no-push    # create the tag locally without pushing
./scripts/create_tag.sh --yes        # skip interactive confirmation
```

- The version bump and the CHANGELOG promotion **must happen in the same
  run**;
- pushing a `v*` tag triggers the `.github/workflows/release.yml` pipeline;
- **never push a real tag from a test environment**.

## 4. Packaging and Installation

`./scripts/build.sh` (cross-platform build):

- `--fast`: release-fast profile (no LTO, fast incremental builds);
- `--dist`: build + package current-platform artifacts (tar.gz / zip) into
  `dist/` (gitignored; verifies the submodule is initialized before
  packaging);
- `--target=<triple>`: cross-compilation; `--install`: install after
  building; `--dry-run`: preview.
- Artifact names look like `zhuye-cli-<version>-<triple>.tar.gz`.

`./scripts/install.sh` (global installation):

- Prefers an existing `target/release/zhuye-cli` (builds automatically if
  missing);
- defaults to `/usr/local/bin`; `--local` → `~/.local/bin`; `--install-dir`
  for a custom location.

## 5. Release Pipeline (release.yml)

The CI (`.github/workflows/release.yml`) only triggers on a `v*` tag and
builds distribution packages in parallel on macOS / Linux / Windows; the
submodule must be checked out recursively. Releases are built with
`build.sh --dist`, which only builds and packages — it does **not** run
tests, which is why the review must complete before the tag is created.
